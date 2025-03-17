use std::io::{self, Read, Write};
use std::panic;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::Result;
use log::{error, info, LevelFilter};
use serde_json::{to_string, Value};
use simplelog::{Config, WriteLogger};
use zeromq::{Socket as _, SocketRecv as _, ZmqMessage};

use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::task;

use nativeext::{ErrorKind, FromBrowser, ToBrowser};

const MSG_LEN_MAX: u32 = 8 * 1024;

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

async fn run_config_handler(tx: Sender<ToBrowser>) {
    let mut socket = zeromq::SubSocket::new();

    // `connect` will wait for server to open
    // It should only return an error for malformed endpoint
    if socket.connect("tcp://127.0.0.1:5556").await.is_err() {
        error!("failed to connect to config server (malformed endpoint)");
        send_response_message(
            &tx,
            ToBrowser::Error {
                req_id: None,
                error: ErrorKind::FailedToConnectConfig,
            },
        )
        .await;
        return;
    }

    socket.subscribe("").await.expect("Failed to subscribe");
    info!("Subscribed to config server");

    loop {
        let message = socket.recv().await.expect("failed to recieve");
        let string = message_to_string(message).expect("failed to convert message to string");

        let (key, value) =
            split_key_value(&string).expect("failed to convert message string to key-value pair");
        let value: Value = serde_json::from_str(value).expect("failed to deserialize value");
        let (key, value) = (key.to_string(), value);

        info!("Update config: `{}={}`", key, value);

        let response = ToBrowser::UpdateConfig { key, value };
        send_response_message(&tx, response).await;
    }
}

async fn run_request_loop(tx: Sender<ToBrowser>, stdin_lock: Arc<Mutex<io::Stdin>>) {
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

async fn send_response(stdout_lock: &mut io::StdoutLock<'_>, response: &ToBrowser) -> Result<()> {
    let response_string = to_string(response)?;
    let response_length = u32::try_from(response_string.len())?;

    write_ne_u32(stdout_lock, response_length)?;
    stdout_lock.write_all(response_string.as_bytes())?;
    stdout_lock.flush()?;
    Ok(())
}

async fn process_message(request: FromBrowser) -> Result<ToBrowser, u64> {
    tokio::time::sleep(random_duration()).await;

    match request {
        FromBrowser::Ping { req_id } => {
            info!("Ping received from browser");
            Ok(ToBrowser::Pong { req_id })
        }

        FromBrowser::GetTextTopics { req_id, url, text } => {
            info!("GetTextTopics received from browser");
            let topics = get_text_topics_dummy(url, text);
            Ok(ToBrowser::ReturnTextTopics { req_id, topics })
        }
    }
}

fn message_to_string(message: ZmqMessage) -> Option<String> {
    let bytes = message.get(0)?.to_vec();
    let string = String::from_utf8(bytes).ok()?;
    Some(string)
}

fn split_key_value(string: &str) -> Option<(&str, &str)> {
    let index = string.find('=')?;
    let (key, value) = string.split_at(index);

    let mut value = value.chars();
    value.next();
    let value = value.as_str();

    let (key, value) = (key.trim(), value.trim());
    if key.is_empty() || value.is_empty() {
        return None;
    }

    Some((key, value))
}

fn get_text_topics_dummy(url: String, text: String) -> Vec<String> {
    let domain = url
        .splitn(2, "://")
        .nth(1)
        .unwrap_or(&url)
        .splitn(2, '/')
        .next()
        .unwrap_or(&url);

    let mut topics = Vec::new();
    topics.push(format!("FROM {}", domain));

    let sentences = text.split('.');
    for sentence in sentences {
        let mut words = sentence
            .split_whitespace()
            .map(|word| word.trim())
            .filter(|word| !word.is_empty());

        let Some(topic) = words.next() else {
            continue;
        };
        let mut topic = topic.to_uppercase();

        if let Some(next) = words.next() {
            topic += " ";
            topic += &next.to_lowercase();
        }

        topics.push(topic);
    }

    topics.push("END".to_string());
    topics
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

fn random_duration() -> Duration {
    let number = std::time::Instant::now().elapsed().as_nanos() % 256;
    let number = number as u64 * 3 + 200;
    Duration::from_millis(number)
}
