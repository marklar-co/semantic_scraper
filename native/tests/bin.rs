use std::fmt::Debug;
use std::io::Write;
use std::process::{Child, Command, Stdio};

use nativeext::{Request, Response};

const BIN_PATH: &str = env!(concat!("CARGO_BIN_EXE_", env!("CARGO_PKG_NAME")));

pub struct BinaryProgram {
    child: Child,
}

impl BinaryProgram {
    pub fn new() -> Self {
        let child = Command::new(BIN_PATH)
            .stdin(Stdio::piped())
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to spawn native extension process");
        Self { child }
    }

    pub fn send<A>(mut self, requests: A) -> Self
    where
        A: IntoIterator<Item = Request> + Send + 'static,
    {
        let mut stdin = self
            .child
            .stdin
            .take()
            .expect("Failed to open stdin of native extension process");

        for request in requests {
            let request = serde_json::to_string(&request).expect("Failed to serialize request");
            let request = encode_message(&request);
            stdin
                .write_all(&request)
                .expect("Failed to write to stdin of native extension process");
        }
        self
    }

    pub fn receive(self) -> Vec<Response> {
        let output = self
            .child
            .wait_with_output()
            .expect("Failed to read stdout of native extension process");
        assert!(
            output.status.success(),
            "Native extension process failed with exit code {}",
            output.status,
        );
        let mut bytes = output.stdout.into_iter().peekable();

        let mut responses = Vec::new();
        while bytes.peek().is_some() {
            let response = decode_message(&mut bytes);
            let response = serde_json::from_str(&response).expect("Failed to serialize response");
            responses.push(response);
        }
        responses
    }

    pub fn expect_responses<B>(self, expected: B)
    where
        B: IntoIterator<Item = Response> + Send + Debug + Clone + 'static,
    {
        let responses = self.receive();
        let mut remaining_responses = responses.clone();

        for response in expected.clone() {
            let Some(index) = remaining_responses.iter().position(|x| x == &response) else {
                println!("Expected: {:#?}", expected);
                println!("Responses: {:#?}", responses);
                panic!("Response does not contain {:#?}", response);
            };
            remaining_responses.remove(index);
        }

        println!("Left over responses: {:#?}", remaining_responses);
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
