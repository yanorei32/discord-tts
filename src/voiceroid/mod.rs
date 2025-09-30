use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use reqwest::header::{HeaderMap, HeaderName};
use serde::Deserialize;
use tap::Tap;

use crate::tts::{CharacterView, StyleView, TtsService};

mod api;

#[derive(Deserialize, Debug, Clone)]
pub struct Setting {
    pub url: reqwest::Url,
    pub headers: HashMap<String, String>,
}

#[derive(Debug)]
struct VoiceroidInner {
    client: reqwest::Client,
    url: reqwest::Url,
    voices: Vec<api::Voice>,
}

#[derive(Clone, Debug)]
pub struct Voiceroid {
    inner: Arc<VoiceroidInner>,
}

impl Voiceroid {
    pub async fn new(setting: &Setting) -> Result<Self> {
        let api_voices = setting.url.clone().tap_mut(|u| {
            u.path_segments_mut().unwrap().push("api").push("voices");
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
            .user_agent("discord-tts-voiceroid/0.0.0")
            .build()
            .unwrap();

        let voices = client
            .get(api_voices)
            .send()
            .await
            .context("Failed to get /api/voices")?
            .error_for_status()
            .context("Failed to get /api/voices")?;

        let voices = voices.json().await.context("Failed to parse /api/voices")?;

        Ok(Voiceroid {
            inner: Arc::new(VoiceroidInner {
                url: setting.url.clone(),
                voices,
                client,
            }),
        })
    }
}

#[async_trait]
impl TtsService for Voiceroid {
    async fn tts(&self, style_id: &str, text: &str) -> Result<Vec<u8>> {
        let api_tts = self.inner.url.clone().tap_mut(|u| {
            u.path_segments_mut().unwrap().push("api").push("tts");
        });

        let (voice_id, style) = style_id.split_once('/').context("Invalid StyleID")?;

        let voice = self
            .inner
            .voices
            .iter()
            .find(|v| v.id == voice_id)
            .context("Invalid CharacterID")?;

        let mut is_kansai = match voice.dialect.as_str() {
            "Standard" => false,
            "Kansai" => true,
            _ => unreachable!(),
        };

        if style == "alt" {
            is_kansai = !is_kansai;
        }

        let query = api::TtsRequest {
            is_kansai,
            text: text.to_string(),
            voice_id: voice_id.to_string(),
        };

        let resp = self
            .inner
            .client
            .post(api_tts)
            .json(&query)
            .send()
            .await
            .context("Failed to post /api/tts (connect)")?
            .error_for_status()
            .context("Failed to post /api/tts (status_code)")
            .unwrap();

        let resp = resp
            .bytes()
            .await
            .context("Failed to post /api/tts (body)")?;

        Ok(resp.to_vec())
    }

    fn is_available(&self, style_id: &str) -> bool {
        let (voice_id, _style) = style_id.split_once('/').context("Invalid StyleID").unwrap();
        self.inner.voices.iter().any(|v| v.id == voice_id)
    }

    fn styles(&self) -> Vec<CharacterView> {
        self.inner
            .voices
            .iter()
            .map(|voice| {
                let (normal, alt) = match voice.dialect.as_str() {
                    "Standard" => ("標準語", "関西弁"),
                    "Kansai" => ("関西弁", "標準語"),
                    _ => unreachable!(),
                };

                let icon = BASE64.decode(&voice.icon).expect("Failed to decode icon");

                let normal = StyleView {
                    name: normal.to_string(),
                    id: format!("{}/normal", voice.id),
                    icon: icon.clone(),
                };

                let alt = StyleView {
                    name: format!("{alt} (強制)"),
                    id: format!("{}/alt", voice.id),
                    icon: icon.clone(),
                };

                CharacterView {
                    name: voice.name.clone(),
                    policy: "VOICEROID利用規約に則り、ご利用ください。".to_string(),
                    styles: vec![normal, alt],
                }
            })
            .collect()
    }
}
