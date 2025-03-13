use std::{
    io::Write,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

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
            &[
                FromBrowser::Ping { req_id: 7 },
                FromBrowser::Ping { req_id: 14 },
                FromBrowser::Ping { req_id: 8 },
                FromBrowser::Ping { req_id: 8 },
                FromBrowser::Ping { req_id: 17 },
                FromBrowser::Ping { req_id: 24 },
                FromBrowser::Ping { req_id: 18 },
                FromBrowser::Ping { req_id: 18 },
            ],
            &[
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

    pub fn send(mut self, messages: &'static [FromBrowser]) -> Self {
        let mut stdin = self.child.stdin.take().expect("Failed to open stdin");
        std::thread::spawn(move || {
            for message in messages {
                let message = serde_json::to_string(message).expect("failed to serialize request");
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

    pub fn expect_response(input: &'static [FromBrowser], expected: &[ToBrowser]) {
        println!("requests: {:#?}", input);

        let mut output = NativeExt::new().send(input).receive();

        for entry in expected {
            let Some(index) = output.iter().position(|x| x == entry) else {
                panic!("response does not contain {:?}", entry);
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
