use std::collections::HashMap;
use std::io::Cursor;
use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use hound::{WavReader, WavWriter};
use reqwest::header::{HeaderMap, HeaderName};
use serde::Deserialize;
use tap::Tap;

use crate::tts::{CharacterView, StyleView, TtsService, split_long_text};

mod api;

fn default_headers() -> HashMap<String, String> {
    HashMap::new()
}

fn default_master_volume() -> f32 {
    1.0
}

fn default_speed() -> f32 {
    1.0
}

fn default_pitch() -> f32 {
    1.0
}

fn default_voices() -> HashMap<String, VoiceConfig> {
    HashMap::new()
}

fn default_max_chars() -> usize {
    500
}

#[derive(Deserialize, Debug, Clone)]
pub struct VoiceConfig {
    pub voice_id: String,
    #[serde(default = "default_speed")]
    pub speed: f32,
    #[serde(default = "default_pitch")]
    pub pitch: f32,
    #[serde(default = "default_master_volume")]
    pub volume: f32,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Setting {
    pub url: reqwest::Url,
    #[serde(default = "default_headers")]
    pub headers: HashMap<String, String>,
    #[serde(default = "default_master_volume")]
    pub master_volume: f32,
    #[serde(default = "default_voices")]
    pub voices: HashMap<String, VoiceConfig>,
    #[serde(default = "default_max_chars")]
    pub max_chars: usize,
}

#[derive(Debug)]
struct AndroidTTSInner {
    client: reqwest::Client,
    url: reqwest::Url,
    master_volume: f32,
    voices: HashMap<String, VoiceConfig>,
    max_chars: usize,
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Clone, Debug)]
pub struct AndroidTTS {
    inner: Arc<AndroidTTSInner>,
}

impl AndroidTTS {
    pub fn new(setting: &Setting) -> Result<Self> {
        let mut headers = HeaderMap::new();

        for (key, value) in &setting.headers {
            headers.insert(
                HeaderName::from_bytes(key.as_bytes()).context("Invalid HeaderName")?,
                value.parse().context("Invalid HeaderValue")?,
            );
        }

        let client = reqwest::ClientBuilder::new()
            .default_headers(headers)
            .user_agent("discord-tts-androidtts/0.0.0")
            .build()
            .unwrap();

        Ok(AndroidTTS {
            inner: Arc::new(AndroidTTSInner {
                url: setting.url.clone(),
                master_volume: setting.master_volume,
                voices: setting.voices.clone(),
                client,
                max_chars: setting.max_chars,
            }),
        })
    }
}

#[async_trait]
impl TtsService for AndroidTTS {
    async fn tts(&self, style_id: &str, text: &str) -> Result<Vec<u8>> {
        let api_tts = self.inner.url.clone().tap_mut(|u| {
            u.path_segments_mut().unwrap().push("api").push("tts");
        });

        let (voice_id, speed, pitch, volume) = if style_id == "default" {
            (None, 1.0, 1.0, self.inner.master_volume)
        } else {
            let voice_config = self.inner.voices.get(style_id).context("Invalid VoiceID")?;
            let speed = voice_config.speed;
            let pitch = voice_config.pitch;
            let volume = voice_config.volume * self.inner.master_volume;
            (Some(voice_config.voice_id.as_str()), speed, pitch, volume)
        };

        let parts = if self.inner.max_chars == 0 {
            vec![text.to_string()]
        } else {
            split_long_text(text, self.inner.max_chars)
        };

        let mut all_samples: Vec<i16> = Vec::new();
        let mut sample_rate = None;

        for part in parts {
            let query = api::TtsRequest {
                text: part,
                voice_id: voice_id.map(String::from),
                #[allow(clippy::float_cmp)]
                speed: if speed == 1.0 { None } else { Some(speed) },
                #[allow(clippy::float_cmp)]
                pitch: if pitch == 1.0 { None } else { Some(pitch) },
            };

            let resp = self
                .inner
                .client
                .post(api_tts.clone())
                .json(&query)
                .send()
                .await
                .context("Failed to post /api/tts (connect)")?
                .error_for_status()
                .context("Failed to post /api/tts (status_code)")?;

            let wav_data = resp
                .bytes()
                .await
                .context("Failed to post /api/tts (body)")?;

            let mut reader = WavReader::new(Cursor::new(wav_data))
                .with_context(|| "Failed to read as wav file")?;
            let spec = reader.spec();
            if sample_rate.is_none() {
                sample_rate = Some(spec.sample_rate);
            }

            for sample in reader.samples::<i16>() {
                all_samples.push(sample?);
            }
        }

        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: sample_rate.unwrap_or(24000),
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut writer =
                WavWriter::new(&mut cursor, spec).with_context(|| "Failed to create wav")?;
            for sample in all_samples {
                let sample = f32::from(sample) * volume;
                let sample = sample.min(f32::from(i16::MAX)).max(f32::from(i16::MIN));
                #[allow(clippy::cast_possible_truncation)]
                writer.write_sample(sample as i16)?;
            }
            writer.finalize()?;
        }

        Ok(cursor.into_inner())
    }

    async fn styles(&self) -> Result<Vec<CharacterView>> {
        let mut styles = vec![];

        if self.inner.voices.is_empty() {
            styles.push(StyleView {
                icon: vec![],
                name: "Default".to_string(),
                id: "default".to_string(),
            });
        }

        for name in self.inner.voices.keys() {
            styles.push(StyleView {
                icon: vec![],
                name: name.clone(),
                id: name.clone(),
            });
        }

        Ok(vec![CharacterView {
            name: "Android TTS".to_string(),
            policy: "Android TTS Engine".to_string(),
            styles,
        }])
    }
}
