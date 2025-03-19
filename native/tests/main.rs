mod bin;
mod utils;

use self::bin::BinaryProgram;
use nativeext::{ErrorKind, Request, Response};
use ntest::timeout;

#[test]
fn ping_many() {
    utils::parallelize(20, ping);
}

#[test]
fn get_topics_many() {
    utils::parallelize(100, get_topics);
}

#[test]
#[timeout(2000)]
fn invalid_topic() {
    BinaryProgram::expect_response(
        [
            Request::GetTextTopics {
                req_id: 7,
                url: "".to_string(),
                text: "".to_string(),
            },
            Request::GetTextTopics {
                req_id: 8,
                url: "".to_string(),
                text: "".to_string(),
            },
        ],
        [
            Response::Error {
                req_id: Some(7),
                error: ErrorKind::ClientProcess,
            },
            Response::Error {
                req_id: Some(8),
                error: ErrorKind::ClientProcess,
            },
        ],
    );
}

#[timeout(8000)]
fn ping() {
    BinaryProgram::expect_response(
        [
            Request::Ping { req_id: 7 },
            Request::Ping { req_id: 14 },
            Request::Ping { req_id: 8 },
            Request::Ping { req_id: 8 },
            Request::Ping { req_id: 17 },
            Request::Ping { req_id: 24 },
            Request::Ping { req_id: 18 },
            Request::Ping { req_id: 18 },
        ],
        [
            Response::Pong { req_id: 7 },
            Response::Pong { req_id: 14 },
            Response::Pong { req_id: 8 },
            Response::Pong { req_id: 8 },
            Response::Pong { req_id: 17 },
            Response::Pong { req_id: 24 },
            Response::Pong { req_id: 18 },
            Response::Pong { req_id: 18 },
        ],
    );
}

#[timeout(8000)]
fn get_topics() {
    BinaryProgram::expect_response(
        [
            Request::GetTextTopics {
                req_id: 7,
                url: "https://lorem.com/index.html".to_string(),
                text: "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.".to_string(),
            },
            Request::GetTextTopics {
                req_id: 2,
                url: "https://emptytext.com".to_string(),
                text: "".to_string(),
            },
            Request::GetTextTopics {
                req_id: 4,
                url: "https://longtext.com".to_string(),
                text: "Lorem ipsum ".repeat(100) + "dolor sit amet.",
            },
            Request::GetTextTopics {
                req_id: 8,
                url: "https://vitae.com".to_string(),
                text: " vitae  interdum, posuere ullamcorper ac ac sit amet justo. curabitur Posuere  et ".to_string(),
            },
            Request::GetTextTopics {
                req_id: 55,
                url: "".to_string(),
                text: "Lorem ipsum".to_string(),
            },
            Request::GetTextTopics {
                req_id: 5,
                url: "https://foreign.lang.com".to_string(),
                text: "这是.一个中文测.试文本".to_string(),
            },
            Request::GetTextTopics {
                req_id: 12,
                url: "https://whitespace.com".to_string(),
                text: "  \n \n ".to_string(),
            },
        ],
        [
            Response::ReturnTextTopics {
                req_id: 7,
                topics: vec![
                    "FROM lorem.com".to_string(),
                    "LOREM ipsum".to_string(),
                    "UT enim".to_string(),
                    "DUIS aute".to_string(),
                    "EXCEPTEUR sint".to_string(),
                ],
            },
            Response::ReturnTextTopics {
                req_id: 2,
                topics: vec![
                    "FROM emptytext.com".to_string(),
                ],
            },
            Response::ReturnTextTopics {
                req_id: 4,
                topics: vec![
                    "FROM longtext.com".to_string(),
                    "LOREM ipsum".to_string(),
                ],
            },
            Response::ReturnTextTopics {
                req_id: 8,
                topics: vec![
                    "FROM vitae.com".to_string(),
                    "VITAE interdum,".to_string(),
                    "CURABITUR posuere".to_string(),
                ],
            },
            Response::Error {
                req_id: Some(55),
                error: ErrorKind::ClientProcess,
            },
            Response::ReturnTextTopics {
                req_id: 5,
                topics: vec![
                    "FROM foreign.lang.com".to_string(),
                    "这是".to_string(),
                    "一个中文测".to_string(),
                    "试文本".to_string(),
                ],
            },
            Response::ReturnTextTopics {
                req_id: 12,
                topics: vec![
                    "FROM whitespace.com".to_string(),
                ],
            },
        ],
    );
}
