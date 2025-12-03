use anyhow::Result;
use reqwest::Url;

const GOOGLE_TTS_MAX_CHARS: usize = 200;

fn split_long_text(text: &str, max_length: usize) -> Result<Vec<String>> {
    // Regex for whitespace including Zero Width No-Break Space and No-Break Space
    let space_regex = regex::Regex::new(r"[\s\u{FEFF}\u{00A0}]").unwrap();

    // Regex for punctuation: !"#$%&'()*+,-./:;<=>?@[\]^_`{|}~
    // We need to escape - (range), ] (end of class), \ (escape char), and [ (start of nested class) inside []
    // In Rust raw string r##...##, backslash is literal.
    // So we need \\ to match backslash, \] to match closing bracket, and \[ to match opening bracket.
    // - should be at the end or escaped.
    // Also added Japanese punctuation: 。、？！「」（）
    let punct_regex =
        regex::Regex::new(r##"[!"#$%&'()*+,\-./:;<=>?@\[\\\]^_`{|}~。、？！「」（）]"##).unwrap();

    let is_space_or_punct = |c: char| -> bool {
        let s = c.to_string();
        space_regex.is_match(&s) || punct_regex.is_match(&s)
    };

    let mut result = Vec::new();
    let mut start = 0;
    let chars: Vec<char> = text.chars().collect();

    loop {
        if chars.len() - start <= max_length {
            result.push(chars[start..].iter().collect());
            break;
        }

        let mut end = start + max_length - 1;

        if is_space_or_punct(chars[end])
            || (end + 1 < chars.len() && is_space_or_punct(chars[end + 1]))
        {
            result.push(chars[start..=end].iter().collect());
            start = end + 1;
            continue;
        }

        let mut found_split = false;
        for i in (start..=end).rev() {
            if is_space_or_punct(chars[i]) {
                end = i;
                found_split = true;
                break;
            }
        }

        if !found_split {
            let str_chunk: String = chars[start..std::cmp::min(start + max_length, chars.len())]
                .iter()
                .collect();
            anyhow::bail!(
                "The word is too long to split into a short text:\n{} ...\n\nTry to split the text by punctuation.",
                str_chunk
            );
        }

        result.push(chars[start..=end].iter().collect());
        start = end + 1;
    }

    Ok(result)
}

pub async fn get_audio_bytes(text: &str, lang: &str, slow: bool, host: &str) -> Result<Vec<u8>> {
    let parts = split_long_text(text, GOOGLE_TTS_MAX_CHARS)?;
    let mut combined_audio = Vec::new();

    for part in parts {
        let url = Url::parse_with_params(
            host,
            &[
                ("ie", "UTF-8"),
                ("q", &part),
                ("tl", lang),
                ("total", "1"),
                ("idx", "0"),
                ("textlen", &part.len().to_string()),
                ("tk", &"0"),
                ("client", "tw-ob"),
                ("ttsspeed", if slow { "0" } else { "1" }),
            ],
        )?;

        let resp = reqwest::get(url).await?.error_for_status()?.bytes().await?;

        combined_audio.extend_from_slice(&resp);
    }

    Ok(combined_audio)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_short_text() {
        let text = "Hello world";
        let parts = split_long_text(text, 200).unwrap();
        assert_eq!(parts, vec!["Hello world"]);
    }

    #[test]
    fn test_split_long_text_spaces() {
        let text = "a".repeat(150) + " " + &"b".repeat(100);
        let parts = split_long_text(&text, 200).unwrap();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], "a".repeat(150) + " ");
        assert_eq!(parts[1], "b".repeat(100));
    }

    #[test]
    fn test_split_long_text_punctuation() {
        let text = "a".repeat(150) + "," + &"b".repeat(100);
        let parts = split_long_text(&text, 200).unwrap();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], "a".repeat(150) + ",");
        assert_eq!(parts[1], "b".repeat(100));
    }

    #[test]
    fn test_split_japanese_text() {
        let text = "あ".repeat(150) + "。" + &"い".repeat(100);
        let parts = split_long_text(&text, 200).unwrap();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], "あ".repeat(150) + "。");
        assert_eq!(parts[1], "い".repeat(100));
    }

    #[test]
    fn test_split_too_long_word() {
        let text = "a".repeat(300);
        let result = split_long_text(&text, 200);
        assert!(result.is_err());
    }
}
