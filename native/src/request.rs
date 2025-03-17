use std::io::{self, Read, Write};
use std::sync::{Arc, Mutex};

use anyhow::Result;
use log::{error, info};
use nativeext::{ErrorKind, FromBrowser, ToBrowser};
use serde_json::to_string;

use tokio::sync::mpsc::Sender;
use tokio::task;

use crate::{dummy, send_response_message};

const MSG_LEN_MAX: u32 = 8 * 1024;

pub async fn run_request_loop(tx: Sender<ToBrowser>, stdin_lock: Arc<Mutex<io::Stdin>>) {
    loop {
        let stdin_lock = stdin_lock.clone();

        let request = match recv_request(&*stdin_lock).unwrap() {
            Request::Ok(request) => request,
            Request::Err => {
                error!("failed to recieve message");
                send_response_message(
                    &tx,
                    ToBrowser::Error {
                        req_id: None,
                        error: ErrorKind::FailedToReceiveRequest,
                    },
                )
                .await;
                continue;
            }
            Request::EOF => {
                info!("End of input");
                break;
            }
        };
        // info!("Recieved request: {:?}", request);

        spawn_request_handler(tx.clone(), request);
    }
}

fn spawn_request_handler(tx: Sender<ToBrowser>, request: FromBrowser) {
    task::spawn(async move {
        let response = match process_message(request).await {
            Ok(response) => response,
            Err(req_id) => {
                error!("Failed to process message");
                send_response_message(
                    &tx,
                    ToBrowser::Error {
                        req_id: Some(req_id),
                        error: ErrorKind::FailedToProcessRequest,
                    },
                )
                .await;
                return;
            }
        };

        info!("sending message...");
        send_response_message(&tx, response).await;
    });
}

enum Request {
    Ok(FromBrowser),
    Err,
    EOF,
}

fn recv_request(stdin_lock: &Mutex<io::Stdin>) -> Result<Request> {
    let mut stdin = stdin_lock.lock().unwrap();

    let Ok(length) = read_ne_u32(&mut *stdin) else {
        return Ok(Request::EOF);
    };

    if length > MSG_LEN_MAX {
        // info!("Message length {length} exceeds max {MSG_LEN_MAX}");
        return Ok(Request::Err);
    }

    let mut buffer = vec![0; length as usize];
    stdin.read_exact(&mut buffer)?;
    // TODO(feat): Return error
    let request: FromBrowser = serde_json::from_slice(&buffer).expect("failed to parse");

    Ok(Request::Ok(request))
}

pub async fn send_response(
    stdout_lock: &mut io::StdoutLock<'_>,
    response: &ToBrowser,
) -> Result<()> {
    let response_string = to_string(response)?;
    let response_length = u32::try_from(response_string.len())?;

    write_ne_u32(stdout_lock, response_length)?;
    stdout_lock.write_all(response_string.as_bytes())?;
    stdout_lock.flush()?;
    Ok(())
}

async fn process_message(request: FromBrowser) -> Result<ToBrowser, u64> {
    dummy::simulate_random_sleep().await;

    match request {
        FromBrowser::Ping { req_id } => {
            info!("Ping received from browser");
            Ok(ToBrowser::Pong { req_id })
        }

        FromBrowser::GetTextTopics { req_id, url, text } => {
            info!("GetTextTopics received from browser");
            let topics = dummy::get_text_topics(url, text);
            Ok(ToBrowser::ReturnTextTopics { req_id, topics })
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

fn read_ne_u32<R>(reader: &mut R) -> io::Result<u32>
where
    R: Read,
{
    let mut buf = [0u8; 4];
    reader.read_exact(&mut buf)?;
    Ok(u32::from_ne_bytes(buf))
}
