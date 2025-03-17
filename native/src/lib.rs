use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
#[derive(Debug, PartialEq)]
pub enum Request {
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
#[derive(Clone, Debug, PartialEq)]
pub enum Response {
    Pong {
        req_id: u64,
    },
    ReturnTextTopics {
        req_id: u64,
        topics: Vec<String>,
    },
    UpdateConfig {
        key: String,
        value: Value,
    },

    Error {
        req_id: Option<u64>,
        error: ErrorKind,
    },
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ErrorKind {
    Stdin,
    Stdout,

    ConfigConnect,
    ConfigSubscribe,
    ConfigReceive,

    ConfigResponseSize,
    ConfigResponseDecode,
    ConfigResponseDeserialize,

    ClientRequestSize,
    ClientRequestDeserialize,

    ClientResponseSend,
    ClientResponseSize,
    ClientResponseSerialize,

    ClientProcess,

    // TODO(feat): Replace instances with real variants
    Generic,
}

impl From<ErrorKind> for Response {
    fn from(error: ErrorKind) -> Self {
        Self::Error {
            req_id: None,
            error,
        }
    }
}
