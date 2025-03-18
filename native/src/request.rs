//! Read from stdin, process requests, and send responses to event loop.
use std::io::{self, Write};

use anyhow::Result;
use log::{error, info};
use nativeext::{ErrorKind, Request, Response};
use serde_json::to_string;
use tokio::io::{AsyncReadExt as _, Stdin};
use tokio::sync::mpsc::Sender;
use tokio::task;

use crate::{dummy, send_response_message};

/// Technical maximum is 1MB as per [Chrome documentation](https://developer.chrome.com/docs/extensions/develop/concepts/native-messaging).
const MAX_REQUEST_LEN: u32 = 8 * 1024;
/// Technical maximum is 4GB as per [Chrome documentation](https://developer.chrome.com/docs/extensions/develop/concepts/native-messaging).
const MAX_RESPONSE_LEN: u32 = 4_000_000_000;

/// Continuously read [`Request`]s from stdin, spawning a worker for each request.
///
/// Loop breaks when stdin reaches EOF.
pub async fn run_handler(tx: Sender<Response>, mut stdin: Stdin) {
    info!("Starting request handler");
    loop {
        let request = match read_request(&mut stdin).await {
            Ok(Some(request)) => request,
            Ok(None) => {
                info!("End of input");
                break;
            }
            Err(error) => {
                error!("Failed to recieve request: {:?}", error);
                send_response_message(&tx, error.into()).await;
                continue;
            }
        };

        let tx = tx.clone();
        task::spawn(async move {
            process_and_send_response(tx, request).await;
        });
    }
}

/// Process a [`Request`] and send [`Response`] to `tx`.
async fn process_and_send_response(tx: Sender<Response>, request: Request) {
    let response = match process_request(request).await {
        Ok(response) => response,
        Err(error) => {
            error!("Failed to process request: {:?}", error);
            send_response_message(&tx, error).await;
            return;
        }
    };

    info!("Sending response: {:?}", response);
    send_response_message(&tx, response).await;
}

/// Write [`Response`] to stdout, encoding as UTF-8 JSON with a leading native-endian `u32` for
/// payload length.
///
/// Must only be called by main event loop.
pub async fn write_response(
    stdout_lock: &mut io::StdoutLock<'_>,
    response: Response,
) -> Result<(), ErrorKind> {
    let response_string = to_string(&response).map_err(|_| ErrorKind::ClientResponseSerialize)?;

    let response_length =
        u32::try_from(response_string.len()).map_err(|_| ErrorKind::ClientResponseSize)?;
    if response_length > MAX_RESPONSE_LEN {
        return Err(ErrorKind::ClientResponseSize);
    }

    write_ne_u32(stdout_lock, response_length).map_err(|_| ErrorKind::Stdout)?;
    stdout_lock
        .write_all(response_string.as_bytes())
        .map_err(|_| ErrorKind::Stdout)?;
    stdout_lock.flush().map_err(|_| ErrorKind::Stdout)?;
    Ok(())
}

/// Read a [`Request`] from stdin, provided that the data is encoded as UTF-8 JSON with a leading
/// native-endian `u32` for payload length.
///
/// Returns `Ok(None)` if stdin reached EOF.
async fn read_request(stdin: &mut Stdin) -> Result<Option<Request>, ErrorKind> {
    let Some(length) = try_read_ne_u32(stdin).await.map_err(|_| ErrorKind::Stdin)? else {
        return Ok(None);
    };

    if length < 1 {
        return Err(ErrorKind::ClientRequestSize);
    }
    if length > MAX_REQUEST_LEN {
        return Err(ErrorKind::ClientRequestSize);
    }

    let mut buffer = vec![0; length as usize];
    if stdin.read_exact(&mut buffer).await.is_err() {
        return Err(ErrorKind::Stdin);
    }

    let Ok(request) = serde_json::from_slice::<Request>(&buffer) else {
        return Err(ErrorKind::ClientRequestDeserialize);
    };

    Ok(Some(request))
}

/// Process a [`Request`] and transform it into a [`Response`].
///
/// Returns `Err(Response::Error { .. })`, for any error case, including invalid user input.
async fn process_request(request: Request) -> Result<Response, Response> {
    dummy::random_sleep().await;

    match request {
        Request::Ping { req_id } => {
            info!("Received request: Ping");
            Ok(Response::Pong { req_id })
        }

        Request::GetTextTopics { req_id, url, text } => {
            info!("Received request: GetTextTopics");
            let topics = dummy::get_text_topics(url, text).map_err(|error| Response::Error {
                req_id: Some(req_id),
                error,
            })?;
            Ok(Response::ReturnTextTopics { req_id, topics })
        }
    }
}

/// Write a `u32` value to a writer, with native endianess.
fn write_ne_u32<W>(writer: &mut W, value: u32) -> io::Result<()>
where
    W: Write,
{
    writer.write_all(&value.to_ne_bytes())
}

/// Read a `u32` value from a reader, with native endianess.
///
/// Returns `Ok(None)` if stdin reached EOF.
async fn try_read_ne_u32(reader: &mut Stdin) -> io::Result<Option<u32>> {
    let mut buf = [0u8; 4];
    if let Err(error) = reader.read_exact(&mut buf).await {
        match error.kind() {
            io::ErrorKind::UnexpectedEof => return Ok(None),
            _ => return Err(error),
        }
    }
    Ok(Some(u32::from_ne_bytes(buf)))
}
