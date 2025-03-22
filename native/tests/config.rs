use std::time::{Duration, Instant};

use zeromq::{Socket as _, SocketSend as _};

pub const KEYS: &[&str] = &["foo", "bar", "baz"];
pub const VALUES: &[&str] = &["4", "\"something\"", "24"];

pub async fn run_config_server(delay: Duration) -> Result<(), Box<dyn std::error::Error>> {
    println!("Start test config server");

    let mut socket = zeromq::PubSocket::new();
    socket.bind("tcp://127.0.0.1:5556").await?;

    loop {
        let key = random_choice(KEYS);
        let value = random_choice(VALUES);

        socket.send(format!("{}={}", key, value).into()).await?;
        tokio::time::sleep(delay).await;
    }
}

fn random_choice<'a>(list: &[&'a str]) -> &'a str {
    // Avoid using a crate or platform-specific code
    // Does not need to be high-quality randomness
    let index = Instant::now().elapsed().as_nanos() as usize % list.len();
    list[index]
}
