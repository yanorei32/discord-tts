use std::collections::HashMap;
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

fn default_languages() -> HashMap<String, String> {
    HashMap::from([
        ("ja".to_string(), "Japanese".to_string()),
        ("ko".to_string(), "Korean".to_string()),
        ("zh-CN".to_string(), "Chinese (Simplified)".to_string()),
        ("zh-TW".to_string(), "Chinese (Traditional)".to_string()),
        ("en".to_string(), "English".to_string()),
    ])
}

#[derive(Deserialize, Debug, Clone)]
pub struct Setting {
    pub host: Url,

    #[serde(default = "default_master_volume")]
    pub master_volume: f32,

    #[serde(default = "default_languages")]
    pub languages: HashMap<String, String>,
}

#[derive(Debug)]
struct GoogleTranslateInner {
    host: Url,
    master_volume: f32,
    languages: HashMap<String, String>,
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
                languages: setting.languages.clone(),
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
        let mut languages: Vec<(String, String)> = self
            .inner
            .languages
            .iter()
            .map(|(code, name)| (code.clone(), name.clone()))
            .collect();
        languages.sort_by_key(|(_, name)| name.to_lowercase());

        Ok(chunk_characters(&languages))
    }
}

fn chunk_characters(languages: &[(String, String)]) -> Vec<CharacterView> {
    const CHUNK_SIZE: usize = 25;
    let chunk_count = languages.len().div_ceil(CHUNK_SIZE);

    languages
        .chunks(CHUNK_SIZE)
        .enumerate()
        .map(|(i, chunk)| CharacterView {
            name: if chunk_count == 1 {
                "Google Translate".to_string()
            } else {
                format!("Google Translate ({}/{})", i + 1, chunk_count)
            },
            policy: "Google Terms of Service".to_string(),
            styles: chunk
                .iter()
                .map(|(id, name)| StyleView {
                    name: name.clone(),
                    id: id.clone(),
                    icon: vec![],
                })
                .collect(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_characters_fits_discord_limits() {
        let languages: Vec<(String, String)> = (0..250)
            .map(|i| (format!("l{i}"), format!("Lang {i}")))
            .collect();

        let characters = chunk_characters(&languages);

        assert_eq!(characters.len(), 10);
        assert!(characters.iter().all(|c| c.styles.len() <= 25));
        assert_eq!(characters[0].name, "Google Translate (1/10)");
        assert_eq!(characters[9].name, "Google Translate (10/10)");
    }

    #[test]
    fn chunk_characters_stays_single_when_few() {
        let languages = vec![("ja".to_string(), "Japanese".to_string())];

        let characters = chunk_characters(&languages);

        assert_eq!(characters.len(), 1);
        assert_eq!(characters[0].name, "Google Translate");
    }
}
