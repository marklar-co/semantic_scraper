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
            .expect("Failed to spawn command");
        Self { child }
    }

    pub fn expect_response<A, B>(input: A, expected: B)
    where
        A: IntoIterator<Item = Request> + Send + Debug + 'static,
        B: IntoIterator<Item = Response> + Send + Debug + Clone + 'static,
    {
        let mut output = BinaryProgram::new().send(input).receive();

        let output_original = output.clone();
        let expected_original = expected.clone();

        for entry in expected {
            let Some(index) = output.iter().position(|x| x == &entry) else {
                println!("expected: {:#?}", expected_original);
                println!("responses: {:#?}", output_original);
                panic!("response does not contain {:#?}", entry);
            };
            output.remove(index);
        }

        println!("left over responses: {:#?}", output);
    }

    pub fn send<A>(mut self, messages: A) -> Self
    where
        A: IntoIterator<Item = Request> + Send + 'static,
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

    pub fn receive(self) -> Vec<Response> {
        let mut bytes = self.receive_raw().into_iter().peekable();
        let mut responses = Vec::new();
        while bytes.peek().is_some() {
            let response = decode_message(&mut bytes);
            let response = serde_json::from_str(&response).expect("invalid json response");
            responses.push(response);
        }
        responses
    }

    fn receive_raw(self) -> Vec<u8> {
        let output = self
            .child
            .wait_with_output()
            .expect("Failed to read stdout");

        assert!(
            output.status.success(),
            "command failed with exit code {}",
            output.status,
        );

        // println!("stderr: ```{}```", String::from_utf8_lossy(&output.stderr));

        output.stdout
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
