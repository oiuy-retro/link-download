//! yt-dlp failures that the download list treats specially.

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Hit {
    RateLimit,
    Age,
    Missing,
    Other(String),
}

pub fn classify(text: &str) -> Hit {
    let lower = text.to_lowercase();
    if lower.contains("requested format is not available") {
        return Hit::Other(error_line(text));
    }
    if lower.contains("http error 429") || lower.contains("http error 402") || lower.contains("too many requests") {
        return Hit::RateLimit;
    }
    if lower.contains("confirm your age")
        || lower.contains("age-restricted")
        || lower.contains("age restricted")
        || lower.contains("inappropriate for some users")
    {
        return Hit::Age;
    }
    if lower.contains("video unavailable")
        || lower.contains("this video is unavailable")
        || lower.contains("has been removed")
        || lower.contains("no longer available")
        || lower.contains("private video")
        || lower.contains("this video is private")
        || lower.contains("account associated with this video has been terminated")
        || lower.contains("http error 404")
    {
        return Hit::Missing;
    }
    Hit::Other(error_line(text))
}

fn error_line(text: &str) -> String {
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || looks_like_dump(line) {
            continue;
        }
        let lower = line.to_lowercase();
        if lower.starts_with("error:") || lower.contains("] error:") {
            return shorten(line);
        }
    }
    let line = text
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !looks_like_dump(line))
        .unwrap_or("The download failed.");
    shorten(line)
}

fn looks_like_dump(line: &str) -> bool {
    let text = line.trim_start();
    text.starts_with('{')
        || text.starts_with('[')
        || text.contains("__yt_dlp_client")
        || text.contains("youtube.com/api/timedtext")
}

fn shorten(line: &str) -> String {
    let mut chars = line.chars();
    let short: String = chars.by_ref().take(180).collect();
    if chars.next().is_some() {
        format!("{short}...")
    } else {
        short
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_the_three_special_failures() {
        assert_eq!(classify("ERROR: unable to download video data: HTTP Error 429: Too Many Requests"), Hit::RateLimit);
        assert_eq!(classify("ERROR: HTTP Error 402: Payment Required"), Hit::RateLimit);
        assert_eq!(
            classify("ERROR: [youtube] abc: Sign in to confirm your age. This video may be inappropriate for some users."),
            Hit::Age
        );
        assert_eq!(classify("ERROR: [youtube] abc: Video unavailable"), Hit::Missing);
        assert_eq!(classify("ERROR: This video has been removed"), Hit::Missing);
        assert!(matches!(classify("ERROR: requested format is not available"), Hit::Other(_)));
    }

    #[test]
    fn dump_json_is_not_the_error_message() {
        let mixed = "WARNING: [youtube:tab] HTTP Error 429: Too Many Requests\n{\"id\":\"x\",\"__yt_dlp_client\":\"tv_samsung\"}\n";
        assert_eq!(classify(mixed), Hit::RateLimit);
        match classify(r#"_yt_dlp_client": "tv_samsung"}. {"ext": "srv2", "url": "https://www.youtube.com/api/timedtext?v=x"}"#) {
            Hit::Other(message) => {
                assert!(!message.contains("timedtext"), "{message}");
                assert!(!message.contains("yt_dlp_client"), "{message}");
            }
            other => panic!("{other:?}"),
        }
    }
}
