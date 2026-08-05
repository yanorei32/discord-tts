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

fn default_languages() -> Vec<Language> {
    vec![
        Language::new("ja", "Japanese"),
        Language::new("ko", "Korean"),
        Language::new("zh-CN", "Chinese (Simplified)"),
        Language::new("zh-TW", "Chinese (Traditional)"),
        Language::new("en", "English"),
    ]
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct Language {
    pub code: String,
    pub name: String,
}

impl Language {
    fn new(code: &str, name: &str) -> Self {
        Language {
            code: code.to_string(),
            name: name.to_string(),
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct Setting {
    pub host: Url,

    #[serde(default = "default_master_volume")]
    pub master_volume: f32,

    #[serde(default = "default_languages")]
    pub languages: Vec<Language>,
}

#[derive(Debug)]
struct GoogleTranslateInner {
    host: Url,
    master_volume: f32,
    languages: Vec<Language>,
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
        let languages: Vec<(String, String)> = self
            .inner
            .languages
            .iter()
            .map(|l| (l.code.clone(), l.name.clone()))
            .collect();

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

#[cfg(test)]
mod config_tests {
    use crate::google_translate::{Language, Setting};

    #[test]
    fn languages_keep_config_order() {
        let toml = r#"
host = "https://translate.google.com"
languages = [
    { code = "ko", name = "Korean" },
    { code = "ja", name = "Japanese" },
    { code = "en", name = "English" },
]
"#;
        let setting: Setting = toml::from_str(toml).unwrap();
        assert_eq!(
            setting.languages,
            vec![
                Language::new("ko", "Korean"),
                Language::new("ja", "Japanese"),
                Language::new("en", "English"),
            ]
        );
    }
}
