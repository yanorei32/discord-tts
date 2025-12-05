use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use serde::Deserialize;

use crate::tts::{CharacterView, StyleView, TtsService};

mod bing_speech_tts;
use bing_speech_tts::{get_audio_bytes, list_voices};

fn default_master_volume() -> f32 {
    1.0
}

#[derive(Deserialize, Debug, Clone)]
pub struct Setting {
    pub host: String,

    #[serde(default = "default_master_volume")]
    pub master_volume: f32,
}

#[derive(Debug)]
struct BingSpeechInner {
    host: String,
    master_volume: f32,
}

#[derive(Clone, Debug)]
pub struct BingSpeech {
    inner: Arc<BingSpeechInner>,
}

impl BingSpeech {
    pub fn new(setting: &Setting) -> Self {
        BingSpeech {
            inner: Arc::new(BingSpeechInner {
                host: setting.host.clone(),
                master_volume: setting.master_volume,
            }),
        }
    }
}

fn parse_friendly_name(friendly_name: &str) -> String {
    let parts: Vec<&str> = friendly_name.split(" - ").collect();
    if parts.len() >= 2 {
        parts[0].trim().to_string()
    } else {
        friendly_name.to_string()
    }
}

const LANGUAGES: &[(&str, &str)] = &[
    ("ja-JP", "Japanese (Japan)"),
    ("ko-KR", "Korean (Korea)"),
    ("zh-CN", "Chinese (Simplified)"),
    ("zh-TW", "Chinese (Traditional)"),
    ("zh-HK", "Chinese (Hong Kong)"),
    ("en-US", "English (United States)"),
    ("en-GB", "English (United Kingdom)"),
    ("en-AU", "English (Australia)"),
    ("en-CA", "English (Canada)"),
    ("en-IN", "English (India)"),
];

#[async_trait]
impl TtsService for BingSpeech {
    async fn tts(&self, style_id: &str, text: &str) -> Result<Vec<u8>> {
        let (locale, voice) = style_id
            .split_once('/')
            .ok_or_else(|| anyhow::anyhow!("Invalid style_id format: {style_id}"))?;
        get_audio_bytes(text, voice, locale, &self.inner.host, self.inner.master_volume).await
    }

    async fn styles(&self) -> Result<Vec<CharacterView>> {
        let voices = list_voices(&self.inner.host).await?;

        let mut characters = Vec::new();

        for (locale, language_name) in LANGUAGES {
            let mut styles: Vec<StyleView> = voices
                .iter()
                .filter(|v| &v.locale == locale)
                .map(|voice| StyleView {
                    name: parse_friendly_name(&voice.friendly_name),
                    id: format!("{}/{}", voice.locale, voice.short_name),
                    icon: vec![],
                })
                .collect();

            if !styles.is_empty() {
                styles.sort_by(|a, b| a.name.cmp(&b.name));
                characters.push(CharacterView {
                    name: (*language_name).to_string(),
                    policy: "Microsoft Services Agreement".to_string(),
                    styles,
                });
            }
        }

        Ok(characters)
    }
}
