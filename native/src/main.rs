mod config;
mod dummy;
mod request;

use std::io;
use std::panic;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use log::{info, LevelFilter};
use simplelog::{Config, WriteLogger};

use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::task;

use crate::config::run_config_handler;
use crate::request::{run_request_loop, send_response};
use nativeext::ToBrowser;

const CHANNEL_CAPACITY: usize = 32;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    WriteLogger::init(LevelFilter::Info, Config::default(), io::stderr())?;

    // Exit main thread on any thread panic
    let default_panic = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        default_panic(info);
        std::process::exit(1);
    }));

    info!("Start nativeext");

    // Create all resources here, even if not shared

    // Use `std` mutex instead of that of `tokio`. Must be not locked across `.await` point
    let stdin_lock = Arc::new(Mutex::new(io::stdin()));

    let stdout_lock = io::stdout().lock();

    let (tx_primary, rx_primary) = mpsc::channel::<ToBrowser>(CHANNEL_CAPACITY);
    let (tx_config, rx_config) = mpsc::channel::<ToBrowser>(CHANNEL_CAPACITY);

    task::spawn(async move {
        run_config_handler(tx_config).await;
    });

    task::spawn(async move {
        run_request_loop(tx_primary, stdin_lock).await;
    });

    run_event_loop(rx_primary, rx_config, stdout_lock).await;

    info!("CLOSING MAIN THREAD");
    Ok(())
}

async fn run_event_loop(
    mut rx_primary: Receiver<ToBrowser>,
    mut rx_config: Receiver<ToBrowser>,
    mut stdout_lock: io::StdoutLock<'_>,
) {
    info!("entering loop...");
    loop {
        let Some(response) = next_message(&mut rx_primary, &mut rx_config).await else {
            info!("EXITING LOOP!");
            break;
        };

        send_response(&mut stdout_lock, &response)
            .await
            .expect("failed to send response");
    }
}

async fn next_message(
    rx_primary: &mut Receiver<ToBrowser>,
    rx_config: &mut Receiver<ToBrowser>,
) -> Option<ToBrowser> {
    tokio::select! {
        Some(response) = rx_primary.recv() => Some(response),
        // Ignore config updates if primary channel is already closed
        Some(response) = rx_config.recv(), if !rx_primary.is_closed() => Some(response),
        else => None,
    }
}

async fn send_response_message(tx: &Sender<ToBrowser>, response: ToBrowser) {
    // TODO(feat): Handle error
    tx.send(response)
        .await
        .expect("failed to send response message");
}
