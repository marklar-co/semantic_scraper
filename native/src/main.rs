use std::io::{self, Read, Write};
use std::ops::ControlFlow;
use std::panic;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use log::{info, LevelFilter};
use serde_json::to_string;
use simplelog::{Config, WriteLogger};
use zeromq::{Socket as _, SocketRecv as _, ZmqMessage};

use tokio::sync::Mutex;
use tokio::task::{self, JoinHandle};

use nativeext::{FromBrowser, ToBrowser};

const MSG_LEN_MAX: u32 = 8 * 1024;

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

    let mut stdin_lock = io::stdin().lock();
    let stdout_lock = Arc::new(Mutex::new(io::stdout()));

    handle_config_updates(&stdout_lock).await;

    let mut handles: Vec<JoinHandle<()>> = Vec::new();

    handle_client_messages(&mut handles, &mut stdin_lock, &stdout_lock).await?;

    for handle in handles {
        handle.await?;
    }

    Ok(())
}

async fn handle_config_updates(stdout_lock: &Arc<Mutex<io::Stdout>>) {
    let stdout_lock = stdout_lock.clone();

    // Task will never complete
    tokio::spawn(async move {
        let mut socket = zeromq::SubSocket::new();
        socket
            .connect("tcp://127.0.0.1:5556")
            .await
            .expect("Failed to connect");

        socket.subscribe("").await.expect("Failed to subscribe");
        info!("Subscribed to config server");

        loop {
            let message = socket.recv().await.expect("failed to recieve");
            let string = message_to_string(message).expect("failed to convert message to string");

            let (key, value) = split_key_value(&string)
                .expect("failed to convert message string to key-value pair");
            let (key, value) = (key.to_string(), value.to_string());

            info!("Update config: `{}={}`", key, value);

            send_response(&*stdout_lock, &ToBrowser::UpdateConfig { key, value })
                .await
                .expect("failed to send response");
        }
    });
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

async fn handle_client_messages(
    handles: &mut Vec<JoinHandle<()>>,
    stdin_lock: &mut io::StdinLock<'_>,
    stdout_lock: &Arc<Mutex<io::Stdout>>,
) -> Result<()> {
    loop {
        let request = match recv_request(stdin_lock).await? {
            Ok(request) => request,
            Err(ControlFlow::Continue(())) => continue,
            Err(ControlFlow::Break(())) => break,
        };
        info!("Recieved request: {:?}", request);

        let stdout_lock = stdout_lock.clone();

        handles.push(task::spawn(async move {
            let response = match process_message(request).await {
                Ok(response) => response,
                Err(err) => {
                    info!("Failed to process message: {}", err);
                    return;
                }
            };

            send_response(&*stdout_lock, &response)
                .await
                .expect("Failed to send response");
        }));
    }

    Ok(())
}

async fn recv_request(
    stdin_lock: &mut io::StdinLock<'_>,
) -> Result<Result<FromBrowser, ControlFlow<()>>> {
    let Ok(length) = read_ne_u32(stdin_lock) else {
        info!("Failed to read message length (probably browser exit)");
        return Ok(Err(ControlFlow::Break(())));
    };

    if length > MSG_LEN_MAX {
        info!("Message length {length} exceeds max {MSG_LEN_MAX}");
        return Ok(Err(ControlFlow::Continue(())));
    }

    let mut buffer = vec![0; length as usize];
    stdin_lock.read_exact(&mut buffer)?;
    let request: FromBrowser = serde_json::from_slice(&buffer).expect("failed to parse");

    Ok(Ok(request))
}

async fn send_response(stdout_lock: &Mutex<io::Stdout>, response: &ToBrowser) -> Result<()> {
    let response_string = to_string(response)?;
    let response_length = u32::try_from(response_string.len())?;

    let mut stdout = stdout_lock.lock().await;

    write_ne_u32(&mut *stdout, response_length)?;
    stdout.write_all(response_string.as_bytes())?;
    stdout.flush()?;
    Ok(())
}

async fn process_message(request: FromBrowser) -> Result<ToBrowser> {
    tokio::time::sleep(random_duration()).await;

    match request {
        FromBrowser::Ping { req_id } => {
            info!("Ping received from browser");
            Ok(ToBrowser::Pong { req_id })
        }

        FromBrowser::GetTextTopics {
            req_id,
            url: _,
            text: _,
        } => {
            let topics: Vec<String> = vec!["topic1".to_string(), "topic2".to_string()];
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

fn random_duration() -> Duration {
    use std::io::Read as _;
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .create(false)
        .open("/dev/urandom")
        .unwrap();
    let mut buf = [0u8; 1];
    file.read_exact(&mut buf).unwrap();
    let number = buf[0];

    let number = number as u64 * 3 + 200;

    Duration::from_millis(number)
}
