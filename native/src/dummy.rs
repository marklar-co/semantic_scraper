use std::time::{Duration, Instant};

use tokio::time;

pub async fn simulate_random_sleep() {
    let number = Instant::now().elapsed().as_nanos() % 256;
    let number = number as u64 * 3 + 200;
    let duration = Duration::from_millis(number);
    time::sleep(duration).await;
}

pub fn get_text_topics(url: String, text: String) -> Vec<String> {
    let domain = url
        .splitn(2, "://")
        .nth(1)
        .unwrap_or(&url)
        .splitn(2, '/')
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

    topics.push("END".to_string());
    topics
}
