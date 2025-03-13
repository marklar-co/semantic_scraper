use serde::{Deserialize, Serialize};

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
#[derive(Debug, PartialEq)]
pub enum ToBrowser {
    Pong { req_id: u64 },
    ReturnTextTopics { req_id: u64, topics: Vec<String> },
    UpdateConf { key: String, val: String },
    Test,
}
