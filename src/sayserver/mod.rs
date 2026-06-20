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

fn default_characters() -> Vec<String> {
    vec![]
}

fn default_max_chars() -> usize {
    500
}

#[derive(Deserialize, Debug, Clone)]
pub struct CharacterConfig {
    pub name: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Setting {
    pub url: reqwest::Url,
    #[serde(default = "default_headers")]
    pub headers: HashMap<String, String>,
    #[serde(default = "default_master_volume")]
    pub master_volume: f32,
    #[serde(default = "default_characters")]
    pub characters: Vec<String>,
    #[serde(default = "default_max_chars")]
    pub max_chars: usize,
}

#[derive(Debug)]
struct SayServerInner {
    client: reqwest::Client,
    url: reqwest::Url,
    master_volume: f32,
    characters: Vec<String>,
    max_chars: usize,
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Clone, Debug)]
pub struct SayServer {
    inner: Arc<SayServerInner>,
}

impl SayServer {
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
            .user_agent("discord-tts-sayserver/0.0.0")
            .build()
            .unwrap();

        Ok(SayServer {
            inner: Arc::new(SayServerInner {
                url: setting.url.clone(),
                master_volume: setting.master_volume,
                characters: setting.characters.clone(),
                client,
                max_chars: setting.max_chars,
            }),
        })
    }
}

#[async_trait]
impl TtsService for SayServer {
    async fn tts(&self, style_id: &str, text: &str) -> Result<Vec<u8>> {
        let api_tts = self.inner.url.clone().tap_mut(|u| {
            u.path_segments_mut().unwrap().push("api").push("synthesis");
        });

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
                name: style_id.to_string(),
            };

            let resp = self
                .inner
                .client
                .post(api_tts.clone())
                .json(&query)
                .send()
                .await
                .context("Failed to post /api/synthesis (connect)")?
                .error_for_status()
                .context("Failed to post /api/synthesis (status_code)")?;

            let wav_data = resp
                .bytes()
                .await
                .context("Failed to post /api/synthesis (body)")?;

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
            sample_rate: sample_rate.unwrap_or(22050),
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut cursor = Cursor::new(Vec::new());

        {
            let mut writer =
                WavWriter::new(&mut cursor, spec).with_context(|| "Failed to create wav")?;
            for sample in all_samples {
                let sample = f32::from(sample) * self.inner.master_volume;
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

        for name in &self.inner.characters {
            styles.push(StyleView {
                icon: vec![],
                name: name.clone(),
                id: name.clone(),
            });
        }

        Ok(vec![CharacterView {
            name: "macOS say".to_string(),
            policy: "Apple macOS利用規約に則り、ご利用ください。".to_string(),
            styles,
        }])
    }
}
