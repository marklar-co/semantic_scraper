use std::time::Duration;

use zeromq::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Start server");
    let mut socket = zeromq::PubSocket::new();
    socket.bind("tcp://127.0.0.1:5556").await?;

    println!("Start sending loop");
    loop {
        let zipcode = random_int(10000, 10010);
        let temperature = random_int(-80, 135);
        let relhumidity = random_int(10, 60);
        socket
            .send(format!("{} {} {}", zipcode, temperature, relhumidity).into())
            .await?;
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

fn random_int(min: i32, max: i32) -> i32 {
    assert!(min <= max);

    use std::io::Read as _;
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .create(false)
        .open("/dev/urandom")
        .unwrap();
    let mut buf = [0u8; 4];
    file.read_exact(&mut buf).unwrap();
    let number = i32::from_be_bytes(buf);

    (number.abs() % (max - min + 1)) + min
}
