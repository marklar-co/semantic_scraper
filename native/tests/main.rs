use std::fmt::Debug;
use std::io::Write;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use nativeext::{FromBrowser, ToBrowser};

#[test]
fn ping_multiple() {
    for _ in 0..4 {
        ping();
    }
}

fn ping() {
    assert_duration(Duration::from_secs(1), || {
        NativeExt::expect_response(
            [
                FromBrowser::Ping { req_id: 7 },
                FromBrowser::Ping { req_id: 14 },
                FromBrowser::Ping { req_id: 8 },
                FromBrowser::Ping { req_id: 8 },
                FromBrowser::Ping { req_id: 17 },
                FromBrowser::Ping { req_id: 24 },
                FromBrowser::Ping { req_id: 18 },
                FromBrowser::Ping { req_id: 18 },
            ],
            [
                ToBrowser::Pong { req_id: 7 },
                ToBrowser::Pong { req_id: 14 },
                ToBrowser::Pong { req_id: 8 },
                ToBrowser::Pong { req_id: 8 },
                ToBrowser::Pong { req_id: 17 },
                ToBrowser::Pong { req_id: 24 },
                ToBrowser::Pong { req_id: 18 },
                ToBrowser::Pong { req_id: 18 },
            ],
        );
    });
}

#[test]
fn get_topics() {
    assert_duration(Duration::from_secs(1), || {
        NativeExt::expect_response(
            [
                FromBrowser::GetTextTopics {
                    req_id: 7,
                    url: "https://lorem.com/index.html".to_string(),
                    text: "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.".to_string(),
                },
                FromBrowser::GetTextTopics {
                    req_id: 2,
                    url: "https://emptytext.com".to_string(),
                    text: "".to_string(),
                },
                FromBrowser::GetTextTopics {
                    req_id: 4,
                    url: "https://longtext.com".to_string(),
                    text: "Lorem ipsum ".repeat(100) + "dolor sit amet.",
                },
                FromBrowser::GetTextTopics {
                    req_id: 8,
                    url: "https://vitae.com".to_string(),
                    text: " vitae  interdum, posuere ullamcorper ac ac sit amet justo. curabitur Posuere  et ".to_string(),
                },
                FromBrowser::GetTextTopics {
                    req_id: 5,
                    url: "https://foreign.lang.com".to_string(),
                    text: "这是.一个中文测.试文本".to_string(),
                },
                FromBrowser::GetTextTopics {
                    req_id: 12,
                    url: "https://whitespace.com".to_string(),
                    text: "  \n \n ".to_string(),
                },
            ],
            [
                ToBrowser::ReturnTextTopics {
                    req_id: 7,
                    topics: vec![
                        "FROM lorem.com".to_string(),
                        "LOREM ipsum".to_string(),
                        "UT enim".to_string(),
                        "DUIS aute".to_string(),
                        "EXCEPTEUR sint".to_string(),
                        "END".to_string(),
                    ],
                },
                ToBrowser::ReturnTextTopics {
                    req_id: 2,
                    topics: vec![
                        "FROM emptytext.com".to_string(),
                        "END".to_string(),
                    ],
                },
                ToBrowser::ReturnTextTopics {
                    req_id: 4,
                    topics: vec![
                        "FROM longtext.com".to_string(),
                        "LOREM ipsum".to_string(),
                        "END".to_string(),
                    ],
                },
                ToBrowser::ReturnTextTopics {
                    req_id: 8,
                    topics: vec![
                        "FROM vitae.com".to_string(),
                        "VITAE interdum,".to_string(),
                        "CURABITUR posuere".to_string(),
                        "END".to_string(),
                    ],
                },
                ToBrowser::ReturnTextTopics {
                    req_id: 5,
                    topics: vec![
                        "FROM foreign.lang.com".to_string(),
                        "这是".to_string(),
                        "一个中文测".to_string(),
                        "试文本".to_string(),
                        "END".to_string(),
                    ],
                },
                ToBrowser::ReturnTextTopics {
                    req_id: 12,
                    topics: vec![
                        "FROM whitespace.com".to_string(),
                        "END".to_string(),
                    ],
                },
            ],
        );
    });
}

fn assert_duration<F>(timeout: Duration, f: F)
where
    F: FnOnce(),
{
    let then = Instant::now();
    f();
    let duration = Instant::now() - then;
    assert!(
        duration < timeout,
        "Function took too long to complete: took {:?}, max {:?}",
        duration,
        timeout,
    );
}

struct NativeExt {
    child: Child,
}

impl NativeExt {
    pub fn new() -> Self {
        let child = Command::new("./target/debug/nativeext")
            .stdin(Stdio::piped())
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to spawn command");
        Self { child }
    }

    pub fn send<A>(mut self, messages: A) -> Self
    where
        A: IntoIterator<Item = FromBrowser> + Send + 'static,
    {
        let mut stdin = self.child.stdin.take().expect("Failed to open stdin");
        std::thread::spawn(move || {
            for message in messages {
                let message = serde_json::to_string(&message).expect("failed to serialize request");
                let message = encode_message(&message);
                stdin.write_all(&message).expect("Failed to write to stdin");
            }
        });
        self
    }

    pub fn receive_raw(self) -> Vec<u8> {
        let output = self
            .child
            .wait_with_output()
            .expect("Failed to read stdout");

        println!("stderr: ```{}```", String::from_utf8_lossy(&output.stderr));

        output.stdout
    }

    pub fn receive(self) -> Vec<ToBrowser> {
        let mut bytes = self.receive_raw().into_iter().peekable();
        let mut responses = Vec::new();
        while bytes.peek().is_some() {
            let response = decode_message(&mut bytes);
            let response = serde_json::from_str(&response).expect("invalid json response");
            responses.push(response);
        }
        responses
    }

    pub fn expect_response<A, B>(input: A, expected: B)
    where
        A: IntoIterator<Item = FromBrowser> + Send + Debug + 'static,
        B: IntoIterator<Item = ToBrowser> + Send + 'static,
    {
        println!("requests: {:#?}", input);

        let mut output = NativeExt::new().send(input).receive();

        println!("responses: {:#?}", output);

        for entry in expected {
            let Some(index) = output.iter().position(|x| x == &entry) else {
                panic!("response does not contain {:#?}", entry);
            };
            println!("response found: {:?}", entry);
            output.remove(index);
        }

        println!("left over responses: {:#?}", output);
    }
}

fn encode_message(message: &str) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(4 + message.len());
    bytes.extend_from_slice(&(message.len() as u32).to_ne_bytes());
    bytes.extend_from_slice(message.as_bytes());
    bytes
}

fn decode_message(stream: &mut impl Iterator<Item = u8>) -> String {
    let mut len: usize = 0;
    for i in 0..4 {
        let byte = stream.next().expect("Expected byte in length");
        len += (byte as usize) << i;
    }

    let mut response = Vec::with_capacity(len);
    for _ in 0..len {
        let byte = stream.next().expect("Expected byte in payload");
        response.push(byte);
    }

    String::from_utf8(response).expect("Invalid payload encoding")
}
