use serde::{Deserialize, Serialize};

/// A request received from the browser.
///
/// All requests must include a request ID.
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

/// A response to send to the browser.
///
/// Includes unsolicited config updates with no request ID.
///
/// Includes an error response with an optional request ID.
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
        value: String,
    },

    Error {
        req_id: Option<u64>,
        error: ErrorKind,
    },
}

/// Any error which the client may find use in knowing.
#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ErrorKind {
    /// An error occurred while trying to read from stdin.
    Stdin,
    /// An error occurred while trying to write to stdout.
    Stdout,

    /// Failed to connect to the config server.
    ConfigConnect,
    /// Failed to subscribe to the config server connection.
    ConfigSubscribe,
    /// Failed to receive a config message.
    ConfigReceive,

    /// Received a config message with an invalid size (empty or too large).
    ConfigMessageSize,
    /// Failed to decode or deserialize a config message into a UTF-8 key/value pair.
    ConfigMessageDeserialize,

    /// Received a client request with an invalid size (empty or too large).
    ClientRequestSize,
    /// Failed to decode or deserialize a client request as UTF-8 JSON.
    ClientRequestDeserialize,

    /// Tried to send a client response with an invalid size (empty or too large).
    ClientResponseSize,
    /// Failed to serialize a client response as JSON.
    ClientResponseSerialize,

    /// Failed to process a client request.
    ClientProcess,
}

impl From<ErrorKind> for Response {
    fn from(error: ErrorKind) -> Self {
        Self::Error {
            req_id: None,
            error,
        }
    }
}
