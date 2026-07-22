use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use base64::prelude::*;
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

fn default_voices() -> Vec<String> {
    Vec::new()
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
    #[serde(default = "default_voices")]
    pub voices: Vec<String>,
    #[serde(default = "default_max_chars")]
    pub max_chars: usize,
}

#[derive(Debug)]
struct VolcengineInner {
    client: reqwest::Client,
    url: reqwest::Url,
    master_volume: f32,
    voices: Vec<String>,
    max_chars: usize,
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Clone, Debug)]
pub struct Volcengine {
    inner: Arc<VolcengineInner>,
}

impl Volcengine {
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
            .build()
            .unwrap();

        Ok(Volcengine {
            inner: Arc::new(VolcengineInner {
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
impl TtsService for Volcengine {
    async fn tts(&self, style_id: &str, text: &str) -> Result<Vec<u8>> {
        let api_tts = self.inner.url.clone().tap_mut(|u| {
            u.path_segments_mut()
                .unwrap()
                .push("crx")
                .push("tts")
                .push("v1");
        });

        let parts = if self.inner.max_chars == 0 {
            vec![text.to_string()]
        } else {
            split_long_text(text, self.inner.max_chars)
        };

        let mut combined_audio = Vec::new();

        for part in parts {
            let query = api::TtsRequest {
                text: part,
                speaker: style_id.to_string(),
            };

            let resp = self
                .inner
                .client
                .post(api_tts.clone())
                .json(&query)
                .send()
                .await
                .context("Failed to post /crx/tts/v1 (connect)")?
                .error_for_status()
                .context("Failed to post /crx/tts/v1 (status_code)")?;

            let resp: api::Response = resp
                .json()
                .await
                .context("Failed to post /crx/tts/v1 (body)")?;

            if let Some(audio) = resp.audio {
                let bin: Vec<u8> = BASE64_STANDARD
                    .decode(audio.data.as_bytes())
                    .context("Failed to parse as base64")?;

                combined_audio.extend_from_slice(&bin);
            }
        }

        if combined_audio.is_empty() {
            return Ok(crate::tts::EMPTY_WAVE.to_vec());
        }

        crate::tts::convert_mp3_to_wav(combined_audio, self.inner.master_volume)
    }

    async fn styles(&self) -> Result<Vec<CharacterView>> {
        let mut styles = vec![];

        for name in &self.inner.voices {
            styles.push(StyleView {
                icon: vec![],
                name: name.clone(),
                id: name.clone(),
            });
        }

        Ok(vec![CharacterView {
            name: "Volcengine Translate (火山翻译)".to_string(),
            policy: "火山翻译 ToS".to_string(),
            styles,
        }])
    }
}
