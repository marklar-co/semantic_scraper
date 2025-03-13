use std::io::{self, Read, Write};
use std::ops::ControlFlow;
use std::panic;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use byteorder::{NativeEndian, ReadBytesExt, WriteBytesExt};
use log::{info, LevelFilter};
use serde_json::to_string;
use simplelog::{Config, WriteLogger};
use zeromq::{Socket as _, SocketRecv as _};

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

    info!("nativeex started");

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

        loop {
            info!("Message");
            let repl = socket.recv().await.expect("failed to recieve");
            info!("Received: {:?}", repl);
            send_response(&*stdout_lock, &ToBrowser::Test)
                .await
                .expect("failed to send response");
        }
    });
}

async fn handle_client_messages(
    handles: &mut Vec<JoinHandle<()>>,
    stdin_lock: &mut io::StdinLock<'_>,
    stdout_lock: &Arc<Mutex<io::Stdout>>,
) -> Result<()> {
    loop {
        info!("next message");
        let request = match recv_request(stdin_lock).await? {
            Ok(request) => request,
            Err(ControlFlow::Continue(())) => continue,
            Err(ControlFlow::Break(())) => break,
        };

        let stdout_lock = stdout_lock.clone();

        handles.push(task::spawn(async move {
            if let Err(e) = process_message(&*stdout_lock, request).await {
                info!("Failed to process message: {e}");
            }
        }));
    }

    Ok(())
}

async fn recv_request(
    stdin_lock: &mut io::StdinLock<'_>,
) -> Result<Result<FromBrowser, ControlFlow<()>>> {
    let Ok(length) = stdin_lock.read_u32::<NativeEndian>() else {
        info!("failed to read message length (probably browser exit)");
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

    stdout.write_u32::<NativeEndian>(response_length)?;
    stdout.write_all(response_string.as_bytes())?;
    stdout.flush()?;
    Ok(())
}

async fn process_message(stdout_lock: &Mutex<io::Stdout>, request: FromBrowser) -> Result<()> {
    tokio::time::sleep(random_duration()).await;

    match request {
        FromBrowser::Ping { req_id } => {
            info!("ping received from browser");
            send_response(stdout_lock, &ToBrowser::Pong { req_id }).await?;
        }
        FromBrowser::GetTextTopics {
            req_id,
            url: _,
            text: _,
        } => {
            let topics: Vec<String> = vec!["topic1".to_string(), "topic2".to_string()];
            send_response(stdout_lock, &ToBrowser::ReturnTextTopics { req_id, topics }).await?;
        }
    }

    Ok(())
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
    info!("sleep duration: {}", number);

    Duration::from_millis(number)
}
