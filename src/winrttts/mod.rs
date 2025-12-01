use std::collections::BTreeMap;
use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderName};
use serde::Deserialize;
use tap::Tap;

use crate::tts::{CharacterView, StyleView, TtsService};

mod api;

fn default_character_volume() -> HashMap<String, f32> {
    HashMap::new()
}

fn default_master_volume() -> f32 {
    1.0
}

#[derive(Deserialize, Debug, Clone)]
pub struct Setting {
    pub url: reqwest::Url,
    pub headers: HashMap<String, String>,

    #[serde(default = "default_master_volume")]
    pub master_volume: f32,

    #[serde(default = "default_character_volume")]
    pub character_volume: HashMap<String, f32>,
}

#[derive(Debug)]
struct WinRTTTSInner {
    registry_base_path: String,
    client: reqwest::Client,
    url: reqwest::Url,
    voices: Vec<api::Voice>,
    master_volume: f32,
    character_volume: HashMap<String, f32>,
}

#[derive(Clone, Debug)]
pub struct WinRTTTS {
    inner: Arc<WinRTTTSInner>,
}

impl WinRTTTS {
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
            .user_agent("discord-tts-winrttts/0.0.0")
            .build()
            .unwrap();

        let voices = client
            .get(api_voices)
            .send()
            .await
            .context("Failed to get /api/voices")?
            .error_for_status()
            .context("Failed to get /api/voices")?;

        let mut voices: Vec<api::Voice> =
            voices.json().await.context("Failed to parse /api/voices")?;

        let first_voice = voices.get(0).context("Failed to get first voice")?;
        let (registry_base_path, _name) = first_voice
            .id
            .rsplit_once('\\')
            .context("Failed to parse registry path")?;
        let registry_base_path = registry_base_path.to_string();

        for voice in voices.iter_mut() {
            let (path, name) = voice
                .id
                .rsplit_once('\\')
                .context("Failed to parse registry path")?;

            if registry_base_path != path {
                anyhow::bail!("Registry base path is not omittable");
            }

            voice.id = name.to_string();
        }

        Ok(WinRTTTS {
            inner: Arc::new(WinRTTTSInner {
                registry_base_path,
                url: setting.url.clone(),
                master_volume: setting.master_volume,
                character_volume: setting.character_volume.clone(),
                voices,
                client,
            }),
        })
    }
}

#[async_trait]
impl TtsService for WinRTTTS {
    async fn tts(&self, style_id: &str, text: &str) -> Result<Vec<u8>> {
        let api_tts = self.inner.url.clone().tap_mut(|u| {
            u.path_segments_mut().unwrap().push("api").push("tts");
        });

        let character_volume = self
            .inner
            .character_volume
            .get(style_id)
            .copied()
            .unwrap_or(1.0);

        let query = api::TtsRequest {
            audio_volume: self.inner.master_volume * character_volume,
            text: text.to_string(),
            voice_id: format!("{}\\{}", self.inner.registry_base_path, style_id),
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

    async fn styles(&self) -> Result<Vec<CharacterView>> {
        let mut styles: BTreeMap<String, Vec<StyleView>> = BTreeMap::new();

        styles.extend(
            self.inner
                .voices
                .iter()
                .map(|v| (v.language.to_string(), vec![])),
        );

        for voice in &self.inner.voices {
            let target = styles.get_mut(&voice.language).unwrap();

            target.push(StyleView {
                name: voice.display_name.to_string(),
                id: voice.id.to_string(),
                icon: vec![],
            });
        }

        Ok(styles
            .into_iter()
            .map(|(language, styles)| CharacterView {
                name: language.to_string(),
                policy: "Microsoft Windows利用規約に則り、ご利用ください".to_string(),
                styles,
            })
            .collect())
    }
}
