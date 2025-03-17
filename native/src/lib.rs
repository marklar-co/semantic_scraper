use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
#[derive(Debug, PartialEq)]
pub enum FromBrowser {
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
pub enum ToBrowser {
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
#[derive(Clone, Debug, PartialEq)]
pub enum ErrorKind {
    FailedToConnectConfig,
    FailedToProcessRequest,
    FailedToReceiveRequest,
}
