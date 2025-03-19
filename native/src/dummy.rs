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

/// Return a list containing the first 2 words of each sentence, the first word being uppercase and
/// the second lowercase.
///
/// Includes the domain name of the url (prefixed with `'FROM '`) at the beginning of the list.
pub fn get_text_topics(url: &str, text: &str) -> Result<Vec<String>, ErrorKind> {
    if url.is_empty() {
        return Err(ErrorKind::ClientProcess);
    }

    // https://something.example.com/some/path -> something.example.com
    let domain = url
        .split_once("://")
        .map(|x| x.1)
        .unwrap_or(url)
        .split('/')
        .next()
        .unwrap_or(url);

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]

    fn get_text_topics_works() {
        let input = [
            (
                "https://lorem.com/index.html",
                "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.",
            ),
            (
                "https://emptytext.com",
                "",
            ),
            (
                "https://longtext.com",
                &("Lorem ipsum ".repeat(100) + "dolor sit amet."),
            ),
            (
                "https://vitae.com",
                " vitae  interdum, posuere ullamcorper ac ac sit amet justo. curabitur Posuere  et ",
            ),
            (
                "https://foreign.lang.com",
                "这是.一个中文测.试文本",
            ),
            (
                "https://whitespace.com",
                "  \n \n ",
            ),
        ];
        let expected = [
            vec![
                "FROM lorem.com".to_string(),
                "LOREM ipsum".to_string(),
                "UT enim".to_string(),
                "DUIS aute".to_string(),
                "EXCEPTEUR sint".to_string(),
            ],
            vec!["FROM emptytext.com".to_string()],
            vec!["FROM longtext.com".to_string(), "LOREM ipsum".to_string()],
            vec![
                "FROM vitae.com".to_string(),
                "VITAE interdum,".to_string(),
                "CURABITUR posuere".to_string(),
            ],
            vec![
                "FROM foreign.lang.com".to_string(),
                "这是".to_string(),
                "一个中文测".to_string(),
                "试文本".to_string(),
            ],
            vec!["FROM whitespace.com".to_string()],
        ];

        assert_eq!(input.len(), expected.len());
        for (i, (url, text)) in input.into_iter().enumerate() {
            let output = get_text_topics(url, text);
            assert_eq!(output, Ok(expected[i].clone()));
        }
    }

    #[test]
    fn get_text_topics_returns_error() {
        assert_eq!(
            get_text_topics("", "Lorem ipsum"),
            Err(ErrorKind::ClientProcess),
        );
    }
}
