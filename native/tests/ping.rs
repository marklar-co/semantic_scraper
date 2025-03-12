use std::{
    io::Write,
    process::{Child, Command, Stdio},
};

#[test]
fn ping() {
    let output = NativeExt::new()
        .send(&[r#"{"type":"Ping","req_id":7}"#])
        .receive_raw();
    assert_eq!(output, encode_message(r#"{"type":"Pong","req_id":7}"#));
}

#[test]
fn ping_multiple() {
    NativeExt::expect_input_output(
        &[
            r#"{"type":"Ping","req_id":7}"#,
            r#"{"type":"Ping","req_id":14}"#,
            r#"{"type":"Ping","req_id":8}"#,
            r#"{"type":"Ping","req_id":8}"#,
        ],
        vec![
            r#"{"type":"Pong","req_id":7}"#,
            r#"{"type":"Pong","req_id":14}"#,
            r#"{"type":"Pong","req_id":8}"#,
            r#"{"type":"Pong","req_id":8}"#,
        ],
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

    pub fn send(mut self, messages: &'static [&'static str]) -> Self {
        let mut stdin = self.child.stdin.take().expect("Failed to open stdin");
        std::thread::spawn(move || {
            for message in messages {
                stdin
                    .write_all(&encode_message(message))
                    .expect("Failed to write to stdin");
            }
        });
        self
    }

    pub fn receive_raw(self) -> Vec<u8> {
        self.child
            .wait_with_output()
            .expect("Failed to read stdout")
            .stdout
    }

    pub fn receive(self) -> Vec<String> {
        let mut bytes = self.receive_raw().into_iter().peekable();
        let mut responses = Vec::new();
        while bytes.peek().is_some() {
            let response = decode_message(&mut bytes);
            responses.push(response);
        }
        responses
    }

    pub fn expect_input_output(input: &'static [&'static str], mut expected: Vec<&'static str>) {
        let mut output = NativeExt::new().send(input).receive();
        output.sort();
        expected.sort();
        assert_eq!(output, expected);
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
