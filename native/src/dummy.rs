//! Placeholder functions and values to demonstrate the native extension functionality.
use std::time::{Duration, Instant};

use nativeext::ErrorKind;
use tokio::time;

pub const CONFIG_SERVER_ADDRESS: &str = "tcp://127.0.0.1:5556";

#[cfg(target_family = "unix")]
pub const LOG_FILEPATH: &str = r"/tmp/nativeext.log";
#[cfg(target_family = "windows")]
pub const LOG_FILEPATH: &str = r"C:\Windows\Temp\nativeext.log";

/// Sleep for a random-enough amount of time.
pub async fn random_sleep() {
    let number = Instant::now().elapsed().as_nanos() % 256;
    let number = number as u64 * 5 + 500;
    let duration = Duration::from_millis(number);
    time::sleep(duration).await;
}

/// Return a list containing the first 2 words of each sentence.
///
/// Includes the domain name of the url at the beginning of the list.
pub fn get_text_topics(url: String, text: String) -> Result<Vec<String>, ErrorKind> {
    if url.is_empty() {
        return Err(ErrorKind::ClientProcess);
    }

    let domain = url
        .split_once("://")
        .map(|x| x.1)
        .unwrap_or(&url)
        .split('/')
        .next()
        .unwrap_or(&url);

    let mut topics = Vec::new();
    topics.push(format!("FROM {}", domain));

    let sentences = text.split('.');
    for sentence in sentences {
        let mut words = sentence
            .split_whitespace()
            .map(|word| word.trim())
            .filter(|word| !word.is_empty());

        let Some(topic) = words.next() else {
            continue;
        };
        let mut topic = topic.to_uppercase();

        if let Some(next) = words.next() {
            topic += " ";
            topic += &next.to_lowercase();
        }

        topics.push(topic);
    }

    Ok(topics)
}
