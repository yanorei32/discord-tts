use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;

use serde::Deserialize;

use crate::tts::{CharacterView, StyleView, TtsService};

mod naver_tts;
use naver_tts::VOICES;
use naver_tts::get_audio_bytes;

fn default_master_volume() -> f32 {
    1.0
}

fn default_speed() -> i32 {
    0
}

#[derive(Deserialize, Debug, Clone)]
pub struct Setting {
    #[serde(default = "default_master_volume")]
    pub master_volume: f32,



    #[serde(default = "default_speed")]
    pub speed: i32,
}

#[derive(Debug)]
struct NaverInner {
    master_volume: f32,

    speed: i32,
}

#[derive(Clone, Debug)]
pub struct Naver {
    inner: Arc<NaverInner>,
}

impl Naver {
    pub fn new(setting: &Setting) -> Self {
        Naver {
            inner: Arc::new(NaverInner {
                master_volume: setting.master_volume,

                speed: setting.speed,
            }),
        }
    }
}

#[async_trait]
impl TtsService for Naver {
    async fn tts(&self, style_id: &str, text: &str) -> Result<Vec<u8>> {
        let voice = VOICES
            .iter()
            .find(|v| v.speaker == style_id)
            .ok_or_else(|| anyhow::anyhow!("Unsupported style: {style_id}"))?;

        let bytes = get_audio_bytes(
            text,
            voice.lang,
            voice.speaker,
            self.inner.speed,
            self.inner.master_volume,
        )
        .await?;

        Ok(bytes)
    }

    async fn styles(&self) -> Result<Vec<CharacterView>> {
        let languages = vec![
            ("ja", "Japanese"),
            ("ko", "Korean"),
            ("en", "English"),
            ("zh", "Chinese"),
            ("es", "Spanish"),
        ];

        let mut character_views = Vec::new();

        for (lang_id, lang_name) in languages {
            let mut styles = Vec::new();

            for voice in VOICES.iter().filter(|v| v.lang == lang_id) {
                styles.push(StyleView {
                    name: voice.name.to_string(),
                    id: voice.speaker.to_string(),
                    icon: vec![],
                });
            }

            if !styles.is_empty() {
                character_views.push(CharacterView {
                    name: lang_name.to_string(),
                    policy: "Naver Terms of Service".to_string(),
                    styles,
                });
            }
        }

        Ok(character_views)
    }
}
