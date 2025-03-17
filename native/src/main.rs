mod config;
mod dummy;
mod log_file;
mod request;

use std::io;

use anyhow::Result;
use log::{error, info, LevelFilter};
use simplelog::{Config, WriteLogger};

use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::task;

use crate::config::run_config_handler;
use crate::log_file::LogFile;
use crate::request::{run_request_loop, send_response};
use nativeext::{ErrorKind, Response};

const CHANNEL_CAPACITY: usize = 32;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    info!("Start nativeext");

    WriteLogger::init(LevelFilter::Info, Config::default(), LogFile::new())?;

    // Create all resources here, even if not shared

    // Cannot lock stdin here, as it would be locked across `.await` point.
    // Stdin must only be read from inside request reader loop
    let stdin = io::stdin();
    // Lock stdout only once, since it must only be written to inside main event loop.
    let stdout = io::stdout().lock();

    // Channel for sending responses from response handler to event loop.
    // Closing of this channel indicates all client requests have been completed and stdin has
    // reached EOF. When this happens, event loop should break.
    let (tx_primary, rx_primary) = mpsc::channel::<Response>(CHANNEL_CAPACITY);
    // Channel for sending config updates from config listener to event loop.
    // This channel should stay open until the end of the program. Closing of this channel
    // indicates an error in the config listener.
    let (tx_config, rx_config) = mpsc::channel::<Response>(CHANNEL_CAPACITY);

    task::spawn(async move {
        run_config_handler(tx_config).await;
    });
    task::spawn(async move {
        run_request_loop(tx_primary, stdin).await;
    });
    run_event_loop(rx_primary, rx_config, stdout).await;

    info!("CLOSING MAIN THREAD");
    Ok(())
}

async fn run_event_loop(
    mut rx_primary: Receiver<Response>,
    mut rx_config: Receiver<Response>,
    mut stdout_lock: io::StdoutLock<'_>,
) {
    info!("entering loop...");
    while let Some(response) = next_message(&mut rx_primary, &mut rx_config).await {
        if let Err(error) = send_response(&mut stdout_lock, response).await {
            error!("failed to send response to client {:?}", error);
            // Problem could be with payload, so try once to send error message to client
            if let Err(error) =
                send_response(&mut stdout_lock, ErrorKind::ClientResponseSend.into()).await
            {
                error!("failed to send error to client {:?}", error);
            }
        }
    }
    info!("EXITING LOOP!");
}

async fn next_message(
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

async fn send_response_message(tx: &Sender<Response>, response: Response) {
    if let Err(error) = tx.send(response).await {
        // Error can only occur due to receiver closing, so don't try to send error message through
        // same channel
        error!(
            "failed to send response message to main event loop {}",
            error,
        );
    }
}
