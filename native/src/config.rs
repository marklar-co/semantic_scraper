use log::{error, info};
use serde_json::Value;
use zeromq::{Socket as _, SocketRecv as _, ZmqMessage};

use tokio::sync::mpsc::Sender;

use crate::send_response_message;
use nativeext::{ErrorKind, ToBrowser};

pub async fn run_config_handler(tx: Sender<ToBrowser>) {
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
