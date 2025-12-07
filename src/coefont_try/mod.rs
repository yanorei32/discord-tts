use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use serde::Deserialize;

use crate::tts::{CharacterView, StyleView, TtsService};

mod coefont_tts;
use coefont_tts::{get_audio_bytes, VOICES};

fn default_master_volume() -> f32 {
    1.0
}

#[derive(Deserialize, Debug, Clone)]
pub struct Setting {
    #[serde(default = "default_master_volume")]
    pub master_volume: f32,
}

#[derive(Debug)]
struct CoefontInner {
    master_volume: f32,
}

#[derive(Clone, Debug)]
pub struct CoefontTry {
    inner: Arc<CoefontInner>,
}

impl CoefontTry {
    pub fn new(setting: &Setting) -> Self {
        CoefontTry {
            inner: Arc::new(CoefontInner {
                master_volume: setting.master_volume,
            }),
        }
    }
}

#[async_trait]
impl TtsService for CoefontTry {
    async fn tts(&self, style_id: &str, text: &str) -> Result<Vec<u8>> {
        get_audio_bytes(text, style_id, self.inner.master_volume).await
    }

    async fn styles(&self) -> Result<Vec<CharacterView>> {
        let languages = vec![
            ("ja", "Japanese"),
            ("en", "English"),
            ("zh", "Chinese"),
        ];

        let mut character_views = Vec::new();

        for (lang_id, lang_name) in languages {
            let mut styles = Vec::new();

            for voice in VOICES.iter().filter(|v| v.lang == lang_id) {
                styles.push(StyleView {
                    name: voice.name.to_string(),
                    id: voice.id.to_string(),
                    icon: vec![],
                });
            }

            if !styles.is_empty() {
                character_views.push(CharacterView {
                    name: lang_name.to_string(),
                    policy: "Coefont Terms of Service".to_string(),
                    styles,
                });
            }
        }

        Ok(character_views)
    }
}
