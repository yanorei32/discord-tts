use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use derivative::Derivative;
use tokio::sync::RwLock;

#[derive(Clone, Debug)]
pub struct StyleView {
    pub icon: Vec<u8>,
    pub name: String,
    pub id: String,
}

#[derive(Clone, Debug)]
pub struct CharacterView {
    pub name: String,
    pub policy: String,
    pub styles: Vec<StyleView>,
}

pub fn split_long_text(text: &str, max_length: usize) -> Vec<String> {
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
            // Force split at max_length
            end = start + max_length - 1;
        }

        result.push(chars[start..=end].iter().collect());
        start = end + 1;
    }

    result
}

pub fn convert_mp3_to_wav(mp3_data: Vec<u8>, gain: f32) -> anyhow::Result<Vec<u8>> {
    use std::io::Cursor;
    use symphonia::core::audio::{AudioBufferRef, Signal};
    use symphonia::core::codecs::DecoderOptions;
    use symphonia::core::formats::FormatOptions;
    use symphonia::core::io::{MediaSourceStream, MediaSourceStreamOptions};
    use symphonia::core::meta::MetadataOptions;
    use symphonia::core::probe::Hint;

    let mss = MediaSourceStream::new(
        Box::new(Cursor::new(mp3_data)),
        MediaSourceStreamOptions::default(),
    );
    let mut hint = Hint::new();
    hint.with_extension("mp3");

    let probed = symphonia::default::get_probe().format(
        &hint,
        mss,
        &FormatOptions::default(),
        &MetadataOptions::default(),
    )?;

    let mut format = probed.format;
    let track = format
        .default_track()
        .ok_or_else(|| anyhow::anyhow!("No track found"))?;
    let mut decoder =
        symphonia::default::get_codecs().make(&track.codec_params, &DecoderOptions::default())?;

    let track_id = track.id;
    let spec = hound::WavSpec {
        channels: 1,        // Google TTS is usually mono
        sample_rate: 24000, // Google TTS is usually 24kHz
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut wav_cursor = Cursor::new(Vec::new());
    let mut wav_writer = hound::WavWriter::new(&mut wav_cursor, spec)?;

    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(symphonia::core::errors::Error::IoError(_)) => break, // End of stream
            Err(e) => return Err(e.into()),
        };

        if packet.track_id() != track_id {
            continue;
        }

        match decoder.decode(&packet) {
            Ok(decoded_packet) => match decoded_packet {
                AudioBufferRef::F32(buf) => {
                    for &sample in buf.chan(0) {
                        let sample = sample * gain * f32::from(i16::MAX);
                        let sample = sample.min(f32::from(i16::MAX)).max(f32::from(i16::MIN));

                        #[allow(clippy::cast_possible_truncation)]
                        wav_writer.write_sample(sample as i16)?;
                    }
                }
                AudioBufferRef::S16(buf) => {
                    for &sample in buf.chan(0) {
                        let sample = f32::from(sample) * gain;
                        let sample = sample.min(f32::from(i16::MAX)).max(f32::from(i16::MIN));

                        wav_writer.write_sample(sample)?;
                    }
                }
                _ => anyhow::bail!("Unsupported audio format"),
            },
            Err(symphonia::core::errors::Error::IoError(_)) => break,
            Err(symphonia::core::errors::Error::DecodeError(_)) => {} // Skip decode errors
            Err(e) => return Err(e.into()),
        }
    }

    wav_writer.finalize()?;
    Ok(wav_cursor.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_short_text() {
        let text = "Hello world";
        let parts = split_long_text(text, 200);
        assert_eq!(parts, vec!["Hello world"]);
    }

    #[test]
    fn test_split_long_text_spaces() {
        let text = "a".repeat(150) + " " + &"b".repeat(100);
        let parts = split_long_text(&text, 200);
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], "a".repeat(150) + " ");
        assert_eq!(parts[1], "b".repeat(100));
    }

    #[test]
    fn test_split_long_text_punctuation() {
        let text = "a".repeat(150) + "," + &"b".repeat(100);
        let parts = split_long_text(&text, 200);
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], "a".repeat(150) + ",");
        assert_eq!(parts[1], "b".repeat(100));
    }

    #[test]
    fn test_split_japanese_text() {
        let text = "あ".repeat(150) + "。" + &"い".repeat(100);
        let parts = split_long_text(&text, 200);
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], "あ".repeat(150) + "。");
        assert_eq!(parts[1], "い".repeat(100));
    }

    #[test]
    fn test_split_too_long_word() {
        let text = "a".repeat(300);
        let parts = split_long_text(&text, 200);
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], "a".repeat(200));
        assert_eq!(parts[1], "a".repeat(100));
    }
}
#[async_trait]
pub trait TtsService: std::fmt::Debug + Send + Sync {
    async fn tts(&self, style_id: &str, text: &str) -> Result<Vec<u8>>;
    async fn styles(&self) -> Result<Vec<CharacterView>>;
}

#[derive(Derivative)]
#[derivative(Debug)]
struct TtsServicesInner {
    #[allow(clippy::type_complexity)]
    services: RwLock<HashMap<String, (Box<dyn TtsService>, Vec<CharacterView>)>>,
}

#[derive(Clone, Debug)]
pub struct TtsServices {
    inner: Arc<TtsServicesInner>,
}

impl TtsServices {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(TtsServicesInner {
                services: RwLock::new(HashMap::new()),
            }),
        }
    }

    pub async fn styles(&self) -> HashMap<String, Vec<CharacterView>> {
        let services = self.inner.services.read().await;

        let mut styles = HashMap::new();

        for (id, (_service, service_styles)) in services.iter() {
            styles.insert(id.clone(), service_styles.clone());
        }

        styles
    }

    pub async fn register(&self, service_id: &str, service: Box<dyn TtsService>) -> Result<()> {
        let mut services = self.inner.services.write().await;

        if services.get(service_id).is_some() {
            anyhow::bail!("'{service_id}' is already taken");
        }

        let styles = service.styles().await?;

        services.insert(service_id.to_owned(), (service, styles));

        Ok(())
    }

    pub async fn is_available(&self, service_id: &str, style_id: &str) -> bool {
        let services = self.inner.services.read().await;

        let Some((_service, styles)) = services.get(service_id) else {
            return false;
        };

        styles
            .iter()
            .flat_map(|s| s.styles.iter())
            .any(|style| style.id == style_id)
    }

    pub async fn tts(&self, service_id: &str, style_id: &str, text: &str) -> Result<Vec<u8>> {
        let services = self.inner.services.read().await;

        let Some((service, _styles)) = services.get(service_id) else {
            anyhow::bail!("'{service_id}' is not registered");
        };

        service.tts(style_id, text).await
    }
}
