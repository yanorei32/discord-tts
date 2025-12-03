use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use reqwest::Url;
use serde::Deserialize;

use crate::tts::{CharacterView, StyleView, TtsService};

mod google_tts;
use google_tts::get_audio_bytes;

fn default_master_volume() -> f32 {
    1.0
}

#[derive(Deserialize, Debug, Clone)]
pub struct Setting {
    pub host: Url,

    #[serde(default = "default_master_volume")]
    pub master_volume: f32,
}

#[derive(Debug)]
struct GoogleTranslateInner {
    host: Url,
    master_volume: f32,
}

#[derive(Clone, Debug)]
pub struct GoogleTranslate {
    inner: Arc<GoogleTranslateInner>,
}

impl GoogleTranslate {
    pub fn new(setting: &Setting) -> Self {
        GoogleTranslate {
            inner: Arc::new(GoogleTranslateInner {
                host: setting.host.clone(),
                master_volume: setting.master_volume,
            }),
        }
    }
}

#[async_trait]
impl TtsService for GoogleTranslate {
    async fn tts(&self, style_id: &str, text: &str) -> Result<Vec<u8>> {
        let bytes = get_audio_bytes(
            text,
            style_id,
            false,
            &self.inner.host,
            self.inner.master_volume,
        )
        .await?;

        Ok(bytes)
    }

    async fn styles(&self) -> Result<Vec<CharacterView>> {
        let languages = vec![
            ("ja", "Japanese"),
            ("ko", "Korean"),
            ("zh-CN", "Chinese (Simplified)"),
            ("zh-TW", "Chinese (Traditional)"),
            ("en", "English"),
        ];

        let styles = languages
            .into_iter()
            .map(|(id, name)| StyleView {
                name: name.to_string(),
                id: id.to_string(),
                icon: vec![],
            })
            .collect();

        Ok(vec![CharacterView {
            name: "Google Translate".to_string(),
            policy: "Google Terms of Service".to_string(),
            styles,
        }])
    }
}
