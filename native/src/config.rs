//! Subscribe to local ZeroMQ config server and send updates to event loop.
use std::convert::Infallible;

use log::{error, info};
use tokio::sync::mpsc::WeakSender;
use zeromq::{Socket as _, SocketRecv as _, SubSocket, ZmqMessage};

use crate::{dummy, send_response_weak};
use nativeext::{ErrorKind, Response};

/// Continuously read config messages from server, sending updates to `tx` as
/// [`Response::UpdateConfig`].
///
/// Does not spawn subsequent tasks.
///
/// Function should not return, while connection is maintained.
pub async fn run_handler(tx: WeakSender<Response>) {
    info!("Starting config handler");
    let mut socket = match connect_socket().await {
        Ok(socket) => socket,
        Err(error) => {
            send_response_weak(&tx, error.into()).await;
            return;
        }
    };
    info!("Subscribed to config server");

    // Loop should never break
    let _: Infallible = loop {
        let (key, value) = match receive_message(&mut socket).await {
            Ok(message) => message,
            Err(error) => {
                error!("Failed to receive config message: {:?}", error);
                send_response_weak(&tx, error.into()).await;
                continue;
            }
        };
        info!("Received config update: `{}={}`", key, value);

        let response = Response::UpdateConfig { key, value };
        send_response_weak(&tx, response).await;
    };
}

/// Connect to config server and subscribe to messages.
async fn connect_socket() -> Result<SubSocket, ErrorKind> {
    let mut socket = zeromq::SubSocket::new();

    // `connect` will wait for server to open.
    // It should only return an error for malformed endpoint
    if let Err(error) = socket.connect(dummy::CONFIG_SERVER_ADDRESS).await {
        error!(
            "Failed to connect to config server (likely malformed endpoint): {:?}",
            error,
        );
        return Err(ErrorKind::ConfigConnect);
    }

    if let Err(error) = socket.subscribe("").await {
        error!("Failed to subscribe to config server: {:?}", error);
        return Err(ErrorKind::ConfigSubscribe);
    }

    Ok(socket)
}

/// Await the next message from the subscriber socket.
///
/// Deserializes the message by splitting into a key/value pair at the first `=` character.
async fn receive_message(socket: &mut SubSocket) -> Result<(String, String), ErrorKind> {
    let message = socket.recv().await.map_err(|_| ErrorKind::ConfigReceive)?;
    let string = message_to_string(message)?;
    // Value could easily be deserialized as a `serde_json::Value` if necessary
    let (key, value) = split_key_value(&string).ok_or(ErrorKind::ConfigMessageDeserialize)?;
    Ok((key.to_string(), value.to_string()))
}

/// Convert a [`ZmqMessage`] into a [`String`].
///
/// Returns `Err` if the message does not contain exactly one frame, or is not valid UTF-8.
fn message_to_string(message: ZmqMessage) -> Result<String, ErrorKind> {
    if message.len() > 1 {
        return Err(ErrorKind::ConfigMessageSize);
    }
    let Some(frame) = message.get(0) else {
        return Err(ErrorKind::ConfigMessageSize);
    };
    let bytes = frame.to_vec();
    let Ok(string) = String::from_utf8(bytes) else {
        return Err(ErrorKind::ConfigMessageDeserialize);
    };
    Ok(string)
}

/// Split a string into a key/value pair at the first `=` character.
///
/// Does not trim whitespace.
///
/// Returns `None` if either key or value is empty, or no `=` character was found.
fn split_key_value(string: &str) -> Option<(&str, &str)> {
    let index = string.find('=')?;
    let (key, value) = string.split_at(index);

    let mut value = value.chars();
    value.next();
    let value = value.as_str();

    if key.is_empty() || value.is_empty() {
        return None;
    }

    Some((key, value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_key_value_works() {
        assert_eq!(split_key_value("abc=def"), Some(("abc", "def")));
        assert_eq!(
            split_key_value("  abc  = def  ghi  \n "),
            Some(("  abc  ", " def  ghi  \n "))
        );
        assert_eq!(split_key_value("abc==def"), Some(("abc", "=def")));
        assert_eq!(split_key_value(""), None);
        assert_eq!(split_key_value("="), None);
        assert_eq!(split_key_value("abc="), None);
        assert_eq!(split_key_value("=def"), None);
        assert_eq!(split_key_value("=="), None);
        assert_eq!(split_key_value("==="), None);
    }
}
