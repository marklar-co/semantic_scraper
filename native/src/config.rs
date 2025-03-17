use log::{error, info};
use zeromq::{Socket as _, SocketRecv as _, SubSocket, ZmqMessage};

use tokio::sync::mpsc::Sender;

use crate::{dummy, send_response_message};
use nativeext::{ErrorKind, Response};

pub async fn run_config_handler(tx: Sender<Response>) {
    let mut socket = zeromq::SubSocket::new();

    // `connect` will wait for server to open.
    // It should only return an error for malformed endpoint
    if socket.connect(dummy::CONFIG_SERVER_ENDPOINT).await.is_err() {
        error!("failed to connect to config server (malformed endpoint)");
        send_response_message(&tx, ErrorKind::ConfigConnect.into()).await;
        return;
    }

    if socket.subscribe("").await.is_err() {
        error!("failed to subscribe to config server");
        send_response_message(&tx, ErrorKind::ConfigSubscribe.into()).await;
        return;
    }

    info!("Subscribed to config server");
    run_config_loop(tx, socket).await;
}

async fn run_config_loop(tx: Sender<Response>, mut socket: SubSocket) -> ! {
    loop {
        let (key, value) = match recv_message(&mut socket).await {
            Ok(message) => message,
            Err(error) => {
                error!("failed to receive message: {:?}", error);
                send_response_message(&tx, error.into()).await;
                continue;
            }
        };

        info!("Update config: `{}={}`", key, value);

        let response = Response::UpdateConfig { key, value };
        send_response_message(&tx, response).await;
    }
}

async fn recv_message(socket: &mut SubSocket) -> Result<(String, String), ErrorKind> {
    let message = socket.recv().await.map_err(|_| ErrorKind::ConfigReceive)?;
    let string = message_to_string(message)?;
    // Value could easily be deserialized as a `serde_json::Value` if necessary
    let (key, value) = split_key_value(&string).ok_or(ErrorKind::ConfigResponseDeserialize)?;
    Ok((key.to_string(), value.to_string()))
}

fn message_to_string(message: ZmqMessage) -> Result<String, ErrorKind> {
    if message.len() > 1 {
        error!("message to big");
        return Err(ErrorKind::ConfigResponseSize);
    }
    let Some(frame) = message.get(0) else {
        error!("empty message");
        return Err(ErrorKind::ConfigResponseSize);
    };
    let bytes = frame.to_vec();
    let Ok(string) = String::from_utf8(bytes) else {
        error!("message is not utf8");
        return Err(ErrorKind::ConfigResponseDecode);
    };
    Ok(string)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_key_value_works() {
        assert_eq!(split_key_value("abc=def"), Some(("abc", "def")));
        assert_eq!(
            split_key_value("  abc  = def  ghi  \n "),
            Some(("abc", "def  ghi"))
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
