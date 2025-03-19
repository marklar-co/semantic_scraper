mod config;
mod dummy;
mod logger;
mod request;

use std::{error, io};

use anyhow::Result;
use log::{error, info, warn};
use tokio::sync::mpsc::{self, Receiver, Sender, WeakSender};
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
    // Lock stdout only once, since it must only be written to inside main event loop
    let stdout_lock = io::stdout().lock();

    // Channel for sending responses from both the client request handler and the config listener.
    // Closing of this channel indicates all client requests have been completed and stdin has
    // reached EOF. When this happens, event loop should break
    let (tx, rx) = mpsc::channel::<Response>(CHANNEL_CAPACITY);
    // Use a weak sender for config update messages.
    // This way the channel can close even while the config listener is active and holding a sender
    let tx_weak = tx.downgrade();

    task::spawn(async move {
        config::run_handler(tx_weak).await;
    });
    task::spawn(async move {
        request::run_handler(tx, stdin).await;
    });
    run_event_loop(rx, stdout_lock).await;

    info!("Closing nativeext");
    Ok(())
}

/// Continuously receive [`Response`]s from a [`Receiver`], writing each to stdout.
///
/// Loop breaks when `rx` has closed (stdin has reached EOF).
async fn run_event_loop(mut rx: Receiver<Response>, mut stdout_lock: io::StdoutLock<'_>) {
    info!("Entering event loop");
    while let Some(response) = rx.recv().await {
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

/// Send a [`Response`] through a [`Sender`], reporting any errors.
async fn send_response(tx: &Sender<Response>, response: Response) {
    if let Err(error) = tx.send(response).await {
        // Error can only occur due to receiver closing, so don't try to send error message through
        // same channel
        error!("Failed to send message to event loop: {}", error);
    }
}

/// Send a [`Response`] through a [`WeakSender`], reporting any errors.
///
/// If the weak sender cannot be upgraded to a [`Sender`], then a warning will be logged.
async fn send_response_weak(tx: &WeakSender<Response>, response: Response) {
    let Some(tx) = tx.upgrade() else {
        warn!("Channel has been closed, cannot send config update");
        return;
    };
    send_response(&tx, response).await;
}
