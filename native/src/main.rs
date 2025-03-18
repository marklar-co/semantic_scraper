mod config;
mod dummy;
mod logger;
mod request;

use std::{error, io};

use anyhow::Result;
use log::{error, info};
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::task;

use crate::request::write_response;
use nativeext::Response;

const CHANNEL_CAPACITY: usize = 32;

#[tokio::main]
async fn main() -> Result<(), Box<dyn error::Error>> {
    info!("Started nativeext");

    logger::register()?;

    // Create all resources here, even if not shared

    // Use non-blocking stdin.
    // Stdin must only be read from inside request reader loop
    let stdin = tokio::io::stdin();
    // Lock stdout only once, since it must only be written to inside main event loop.
    let stdout_lock = io::stdout().lock();

    // Channel for sending responses from response handler to event loop.
    // Closing of this channel indicates all client requests have been completed and stdin has
    // reached EOF. When this happens, event loop should break.
    let (tx_primary, rx_primary) = mpsc::channel::<Response>(CHANNEL_CAPACITY);
    // Channel for sending config updates from config listener to event loop.
    // This channel should stay open until the end of the program. Closing of this channel
    // indicates an error in the config listener.
    let (tx_config, rx_config) = mpsc::channel::<Response>(CHANNEL_CAPACITY);

    task::spawn(async move {
        config::run_handler(tx_config).await;
    });
    task::spawn(async move {
        request::run_handler(tx_primary, stdin).await;
    });
    run_event_loop(rx_primary, rx_config, stdout_lock).await;

    info!("Closing nativeext");
    Ok(())
}

/// Continuously receive messages from two channels, and write responses to stdout.
///
/// Loop breaks when `rx_primary` has closed (stdin has reached EOF).
async fn run_event_loop(
    mut rx_primary: Receiver<Response>,
    mut rx_config: Receiver<Response>,
    mut stdout_lock: io::StdoutLock<'_>,
) {
    info!("Entering event loop");
    while let Some(response) = receive_next_message(&mut rx_primary, &mut rx_config).await {
        write_response_or_error(&mut stdout_lock, response).await;
    }
    info!("Exiting event loop");
}

/// Try to write a [`Response`] to stdout.
///
/// If anything fails, try to write [`Response::Error`] to stdout instead.
///
/// If the second write fails, give up.
async fn write_response_or_error(stdout_lock: &mut io::StdoutLock<'_>, response: Response) {
    let Err(error) = write_response(stdout_lock, response).await else {
        return;
    };
    error!("Failed to send response to client: {:?}", error);

    // Problem could be with payload, so try once to send error message to client
    let Err(error) = write_response(stdout_lock, error.into()).await else {
        return;
    };
    error!("Failed to send error to client: {:?}", error);
}

/// Read the next message from two channels, prioritizing `rx_primary`.
///
/// Returns `None` iff `rx_primary` has closed. In this case, all messages from `rx_config` will be ignored.
async fn receive_next_message(
    rx_primary: &mut Receiver<Response>,
    rx_config: &mut Receiver<Response>,
) -> Option<Response> {
    tokio::select! {
        Some(response) = rx_primary.recv() => Some(response),
        // Ignore config updates if primary channel is already closed
        Some(response) = rx_config.recv(), if !rx_primary.is_closed() => Some(response),
        // Primary channel is closed
        else => None,
    }
}

/// Send a [`Response`] to a channel, reporting any errors.
async fn send_response_message(tx: &Sender<Response>, response: Response) {
    if let Err(error) = tx.send(response).await {
        // Error can only occur due to receiver closing, so don't try to send error message through
        // same channel
        error!("Failed to send message to event loop: {}", error);
    }
}
