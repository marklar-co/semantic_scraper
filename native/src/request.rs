use std::io::{self, Read, Write};

use anyhow::Result;
use log::{error, info};
use nativeext::{ErrorKind, Request, Response};
use serde_json::to_string;

use tokio::sync::mpsc::Sender;
use tokio::task;

use crate::{dummy, send_response_message};

/// Technical maximum is 1MB as per [Chrome documentation](https://developer.chrome.com/docs/extensions/develop/concepts/native-messaging).
const MAX_REQUEST_LEN: u32 = 8 * 1024;
/// Technical maximum is 4GB as per [Chrome documentation](https://developer.chrome.com/docs/extensions/develop/concepts/native-messaging).
const MAX_RESPONSE_LEN: u32 = 4_000_000_000;

pub async fn run_request_loop(tx: Sender<Response>, mut stdin: io::Stdin) {
    // Cannot lock stdin here, as it would be locked across `.await` point.
    // Lock stdin in each (synchronous) `recv_request` call
    loop {
        let request = match recv_request(&mut stdin) {
            Ok(Some(request)) => request,
            Ok(None) => {
                info!("End of input");
                break;
            }
            Err(error) => {
                error!("failed to recieve message {:?}", error);
                send_response_message(&tx, error.into()).await;
                continue;
            }
        };
        // info!("Recieved request: {:?}", request);

        let tx = tx.clone();
        task::spawn(async move {
            run_request_handler(tx, request).await;
        });
    }
}

async fn run_request_handler(tx: Sender<Response>, request: Request) {
    let response = match process_message(request).await {
        Ok(response) => response,
        Err(req_id) => {
            error!("Failed to process message");
            send_response_message(
                &tx,
                Response::Error {
                    req_id: Some(req_id),
                    error: ErrorKind::ClientProcess,
                },
            )
            .await;
            return;
        }
    };

    info!("sending message...");
    send_response_message(&tx, response).await;
}

/// Returns `Ok(None)` if stdin reached EOF.
fn recv_request(stdin: &mut io::Stdin) -> Result<Option<Request>, ErrorKind> {
    // Lock once, for the duration of this function, rather than at each 'read' call
    let mut stdin = stdin.lock();

    let Some(length) = try_read_ne_u32(&mut stdin).map_err(|_| ErrorKind::Stdin)? else {
        return Ok(None);
    };

    if length > MAX_REQUEST_LEN {
        error!("Message length {length} exceeds max {MAX_REQUEST_LEN}");
        return Err(ErrorKind::ClientRequestSize);
    }

    let mut buffer = vec![0; length as usize];
    if let Err(error) = stdin.read_exact(&mut buffer) {
        error!("failed to read from stdin {:?}", error);
        return Err(ErrorKind::Stdin);
    }

    let Ok(request) = serde_json::from_slice::<Request>(&buffer) else {
        error!("failed to deserialize request");
        return Err(ErrorKind::ClientRequestDeserialize);
    };

    Ok(Some(request))
}

pub async fn send_response(
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

// TODO(feat): Simulate error while processing
async fn process_message(request: Request) -> Result<Response, u64> {
    dummy::random_sleep().await;

    match request {
        Request::Ping { req_id } => {
            info!("Ping received from browser");
            Ok(Response::Pong { req_id })
        }

        Request::GetTextTopics { req_id, url, text } => {
            info!("GetTextTopics received from browser");
            let topics = dummy::get_text_topics(url, text);
            Ok(Response::ReturnTextTopics { req_id, topics })
        }
    }
}

fn write_ne_u32<W>(writer: &mut W, value: u32) -> io::Result<()>
where
    W: Write,
{
    let buf = value.to_ne_bytes();
    writer.write_all(&buf)
}

/// Returns `Ok(None)` if stdin reached EOF.
fn try_read_ne_u32<R>(reader: &mut R) -> io::Result<Option<u32>>
where
    R: Read,
{
    let mut buf = [0u8; 4];
    if let Err(error) = reader.read_exact(&mut buf) {
        match error.kind() {
            io::ErrorKind::UnexpectedEof => return Ok(None),
            _ => return Err(error),
        }
    }
    Ok(Some(u32::from_ne_bytes(buf)))
}
