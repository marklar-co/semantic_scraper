use log::{error, info};
use serde_json::Value;
use zeromq::{Socket as _, SocketRecv as _, SubSocket, ZmqMessage};

use tokio::sync::mpsc::Sender;

use crate::{dummy, send_response_message};
use nativeext::{ErrorKind, Response};

pub async fn run_config_handler(tx: Sender<Response>) {
    let mut socket = zeromq::SubSocket::new();

    // `connect` will wait for server to open
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

        let response = Response::UpdateConfig {
            key: key.to_string(),
            value,
        };
        send_response_message(&tx, response).await;
    }
}

async fn recv_message(socket: &mut SubSocket) -> Result<(String, Value), ErrorKind> {
    let message = socket.recv().await.map_err(|_| ErrorKind::ConfigReceive)?;
    let string = message_to_string(message)?;
    let (key, value) = deserialize_message(&string).ok_or(ErrorKind::ConfigResponseDeserialize)?;
    Ok((key.to_string(), value))
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

fn deserialize_message(string: &str) -> Option<(&str, Value)> {
    let (key, value) = split_key_value(&string)?;
    let value: Value = serde_json::from_str(value).ok()?;
    Some((key, value))
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
    use serde_json::Number;

    use super::*;

    #[test]
    fn deserialize_message_works() {
        assert_eq!(
            deserialize_message(r#"abc="def""#),
            Some(("abc", Value::String("def".to_string()))),
        );
        assert_eq!(
            deserialize_message("  abc  = \"def\"  \n "),
            Some(("abc", Value::String("def".to_string()))),
        );
        assert_eq!(
            deserialize_message(r#"abc=123"#),
            Some(("abc", Value::Number(Number::from_u128(123).unwrap()))),
        );
        assert_eq!(
            deserialize_message(r#"abc=null"#),
            Some(("abc", Value::Null)),
        );
        assert_eq!(deserialize_message("abc=def"), None);
        assert_eq!(deserialize_message("  abc  = def  ghi  \n "), None);
        assert_eq!(deserialize_message("abc==def"), None);
        assert_eq!(deserialize_message(""), None);
        assert_eq!(deserialize_message("="), None);
        assert_eq!(deserialize_message("abc="), None);
        assert_eq!(deserialize_message("=def"), None);
        assert_eq!(deserialize_message("=="), None);
        assert_eq!(deserialize_message("==="), None);
    }

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
