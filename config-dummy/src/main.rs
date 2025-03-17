use std::time::Duration;

use rand::{Rng as _, rngs::ThreadRng};
use serde_json::Value;
use zeromq::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Start server");
    let mut socket = zeromq::PubSocket::new();
    socket.bind("tcp://127.0.0.1:5556").await?;

    let mut rng = rand::rng();

    println!("Start sending loop");
    loop {
        let key = random_choice(&mut rng, &["foo", "bar", "baz"]);

        let values = [Value::from(4), Value::from("something"), Value::from(24)];
        let value = random_choice(&mut rng, &values);

        socket.send(format!("{}={}", key, value).into()).await?;
        tokio::time::sleep(Duration::from_millis(2000)).await;
    }
}

fn random_choice<'a, T>(rng: &mut ThreadRng, list: &'a [T]) -> &'a T {
    &list[rng.random_range(0..list.len())]
}
