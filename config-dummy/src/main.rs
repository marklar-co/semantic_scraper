use std::time::Duration;

use zeromq::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Start server");
    let mut socket = zeromq::PubSocket::new();
    socket.bind("tcp://127.0.0.1:5556").await?;

    println!("Start sending loop");
    loop {
        let key = random_choice(&["foo", "bar", "baz"]);
        let value = random_choice(&[4, 824, 28, 2]);
        socket.send(format!("{}={}", key, value).into()).await?;
        tokio::time::sleep(Duration::from_millis(2000)).await;
    }
}

fn random_choice<T>(list: &[T]) -> &T {
    &list[random_int(0, list.len() - 1)]
}

fn random_int(min: usize, max: usize) -> usize {
    assert!(min <= max);

    use std::io::Read as _;
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .create(false)
        .open("/dev/urandom")
        .unwrap();
    let mut buf = [0u8; size_of::<usize>()];
    file.read_exact(&mut buf).unwrap();
    let number = usize::from_be_bytes(buf);

    number % (max - min + 1) + min
}
