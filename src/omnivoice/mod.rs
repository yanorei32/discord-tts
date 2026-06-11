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

fn default_voice_volumes() -> HashMap<String, f32> {
    HashMap::new()
}

fn default_voice_speeds() -> HashMap<String, f32> {
    HashMap::new()
}

fn default_max_chars() -> usize {
    500
}

#[derive(Deserialize, Debug, Clone)]
pub struct Setting {
    pub url: reqwest::Url,
    #[serde(default = "default_headers")]
    pub headers: HashMap<String, String>,
    #[serde(default = "default_master_volume")]
    pub master_volume: f32,
    #[serde(default = "default_voice_volumes")]
    pub voice_volumes: HashMap<String, f32>,
    #[serde(default = "default_voice_speeds")]
    pub voice_speeds: HashMap<String, f32>,
    #[serde(default = "default_max_chars")]
    pub max_chars: usize,
}

#[derive(Debug)]
struct OmniVoiceInner {
    client: reqwest::Client,
    url: reqwest::Url,
    master_volume: f32,
    voices: Vec<api::Voice>,
    voice_volumes: HashMap<String, f32>,
    voice_speeds: HashMap<String, f32>,
    max_chars: usize,
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Clone, Debug)]
pub struct OmniVoice {
    inner: Arc<OmniVoiceInner>,
}

impl OmniVoice {
    pub async fn new(setting: &Setting) -> Result<Self> {
        let api_voices = setting.url.clone().tap_mut(|u| {
            u.path_segments_mut().unwrap().push("v1").push("voices");
        });

        let mut headers = HeaderMap::new();

        for (key, value) in &setting.headers {
            headers.insert(
                HeaderName::from_bytes(key.as_bytes()).context("Invalid HeaderName")?,
                value.parse().context("Invalid HeaderValue")?,
            );
        }

        let client = reqwest::ClientBuilder::new()
            .default_headers(headers)
            .user_agent("discord-tts-omnivoice/0.0.0")
            .build()
            .unwrap();

        let voices = client
            .get(api_voices)
            .send()
            .await
            .context("Failed to get /v1/voices")?
            .error_for_status()
            .context("Failed to get /v1/voices")?;

        let voices: api::Voices = voices.json().await.context("Failed to parse /v1/voices")?;

        Ok(OmniVoice {
            inner: Arc::new(OmniVoiceInner {
                url: setting.url.clone(),
                master_volume: setting.master_volume,
                voices: voices.voices.clone(),
                client,
                max_chars: setting.max_chars,
                voice_volumes: setting.voice_volumes.clone(),
                voice_speeds: setting.voice_speeds.clone(),
            }),
        })
    }
}

#[async_trait]
impl TtsService for OmniVoice {
    async fn tts(&self, style_id: &str, text: &str) -> Result<Vec<u8>> {
        let api_tts = self.inner.url.clone().tap_mut(|u| {
            u.path_segments_mut().unwrap().push("v1").push("tts");
        });

        let volume =
            *self.inner.voice_volumes.get(style_id).unwrap_or(&1.0) * self.inner.master_volume;
        let speed = *self.inner.voice_speeds.get(style_id).unwrap_or(&1.0);

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
                voice_id: style_id.to_string(),
                speed,
            };

            let resp = self
                .inner
                .client
                .post(api_tts.clone())
                .json(&query)
                .send()
                .await
                .context("Failed to post /v1/tts (connect)")?
                .error_for_status()
                .context("Failed to post /v1/tts (status_code)")?;

            let wav_data = resp
                .bytes()
                .await
                .context("Failed to post /v1/tts (body)")?;

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
        Ok(self
            .inner
            .voices
            .iter()
            .map(|voice| CharacterView {
                name: voice.name.clone(),
                policy: "OmniVoice Engine".to_string(),
                styles: vec![StyleView {
                    icon: vec![],
                    name: "default".to_string(),
                    id: voice.voice_id.clone(),
                }],
            })
            .collect())
    }
}
