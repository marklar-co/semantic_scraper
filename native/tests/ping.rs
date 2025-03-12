use assert_cmd::Command;

#[test]
fn ping() {
    let mut cmd = Command::cargo_bin("nativeext").unwrap();
    cmd.write_stdin(encode_message(r#"{"type":"Ping","req_id":7}"#));
    cmd.assert()
        .success()
        .stdout(encode_message(r#"{"type":"Pong","req_id":7}"#));
}

fn encode_message(message: &str) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(4 + message.len());
    bytes.extend_from_slice(&(message.len() as u32).to_ne_bytes());
    bytes.extend_from_slice(message.as_bytes());
    println!("{:#?}", bytes);
    bytes
}
