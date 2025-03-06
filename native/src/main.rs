use std::fs::File;
use std::io::{self, Read, Write};

use anyhow::Result;
use byteorder::{NativeEndian, ReadBytesExt, WriteBytesExt};
use log::{info, LevelFilter};
use serde::{Deserialize, Serialize};
use serde_json::to_string;
use simplelog::{Config, WriteLogger};

const MSG_LEN_MAX: u32 = 8 * 1024;

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
#[derive(Debug)]
enum FromBrowser {
    Ping {
        req_id: u64,
    },
    GetTextTopics {
        req_id: u64,
        url: String,
        text: String,
    },
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
#[derive(Debug)]
enum ToBrowser {
    Pong { req_id: u64 },
    ReturnTextTopics { req_id: u64, topics: Vec<String> },
    UpdateConf { key: String, val: String },
}

#[cfg(target_family = "unix")]
const LOG_FILEPATH: &str = r"/home/darcy/code/semantic_collector/native/hello.log";
#[cfg(target_family = "windows")]
const LOG_FILEPATH: &str = r"C:\dev\semantic_collector\native\hello.log";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    WriteLogger::init(
        LevelFilter::Info,
        Config::default(),
        File::create(LOG_FILEPATH)?,
    )?;

    info!("nativeex started");

    let stdin = io::stdin();
    let mut stdin_lock = stdin.lock();

    let stdout = io::stdout();
    let mut stdout_lock = stdout.lock();

    loop {
        let Ok(length) = stdin_lock.read_u32::<NativeEndian>() else {
            info!("failed to read message length (probably browser exit)");
            break;
        };

        if length > MSG_LEN_MAX {
            info!("Message length {length} exceeds max {MSG_LEN_MAX}");
            continue;
        }

        let mut buffer = vec![0; length as usize];
        stdin_lock.read_exact(&mut buffer)?;
        let request: Result<FromBrowser, _> = serde_json::from_slice(&buffer);

        if let Ok(message) = request {
            if let Err(e) = process_message(&mut stdout_lock, message) {
                info!("Failed to process message: {e}");
            }
        } else {
            match std::str::from_utf8(&buffer) {
                Ok(buffer_str) => info!("failed to parse FromBrowser: {}", buffer_str),
                Err(_) => info!("failed to parse FromBrowser: {:?}", buffer),
            }
            continue;
        }
    }

    Ok(())
}

fn process_message(stdout_lock: &mut std::io::StdoutLock, request: FromBrowser) -> Result<()> {
    match request {
        FromBrowser::Ping { req_id } => {
            info!("ping received from browser");
            send_response(stdout_lock, &ToBrowser::Pong { req_id })?;
        }
        FromBrowser::GetTextTopics {
            req_id,
            url: _,
            text: _,
        } => {
            let topics: Vec<String> = vec!["topic1".to_string(), "topic2".to_string()];
            send_response(stdout_lock, &ToBrowser::ReturnTextTopics { req_id, topics })?;
        }
    }

    Ok(())
}

fn send_response(stdout_lock: &mut std::io::StdoutLock, response: &ToBrowser) -> Result<()> {
    let response_string = to_string(response)?;
    let response_length = u32::try_from(response_string.len())?;

    stdout_lock.write_u32::<NativeEndian>(response_length)?;
    stdout_lock.write_all(response_string.as_bytes())?;
    stdout_lock.flush()?;
    Ok(())
}
