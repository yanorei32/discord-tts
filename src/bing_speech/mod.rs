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

fn default_languages() -> Vec<Language> {
    vec![
        Language::new("ja-JP", "Japanese (Japan)"),
        Language::new("ko-KR", "Korean (Korea)"),
        Language::new("zh-CN", "Chinese (Simplified)"),
        Language::new("zh-TW", "Chinese (Traditional)"),
        Language::new("zh-HK", "Chinese (Hong Kong)"),
        Language::new("en-US", "English (United States)"),
        Language::new("en-GB", "English (United Kingdom)"),
        Language::new("en-AU", "English (Australia)"),
        Language::new("en-CA", "English (Canada)"),
        Language::new("en-IN", "English (India)"),
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
    #[serde(default = "default_master_volume")]
    pub master_volume: f32,

    #[serde(default = "default_languages")]
    pub languages: Vec<Language>,
}

#[derive(Debug)]
struct BingSpeechInner {
    master_volume: f32,
    languages: Vec<Language>,
}

#[derive(Clone, Debug)]
pub struct BingSpeech {
    inner: Arc<BingSpeechInner>,
}

impl BingSpeech {
    pub fn new(setting: &Setting) -> Self {
        BingSpeech {
            inner: Arc::new(BingSpeechInner {
                master_volume: setting.master_volume,
                languages: setting.languages.clone(),
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

#[async_trait]
impl TtsService for BingSpeech {
    async fn tts(&self, style_id: &str, text: &str) -> Result<Vec<u8>> {
        let (locale, voice) = style_id
            .split_once('/')
            .ok_or_else(|| anyhow::anyhow!("Invalid style_id format: {style_id}"))?;
        get_audio_bytes(text, voice, locale, self.inner.master_volume).await
    }

    async fn styles(&self) -> Result<Vec<CharacterView>> {
        let voices = list_voices().await?;

        let mut characters = Vec::new();

        for language in &self.inner.languages {
            let mut styles: Vec<StyleView> = voices
                .iter()
                .filter(|v| v.locale == language.code)
                .map(|voice| StyleView {
                    name: parse_friendly_name(&voice.friendly_name),
                    id: format!("{}/{}", voice.locale, voice.short_name),
                    icon: vec![],
                })
                .collect();

            if !styles.is_empty() {
                styles.sort_by(|a, b| a.name.cmp(&b.name));
                characters.push(CharacterView {
                    name: language.name.clone(),
                    policy: "Microsoft Services Agreement".to_string(),
                    styles,
                });
            }
        }

        Ok(characters)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn languages_keep_config_order() {
        let toml = r#"
languages = [
    { code = "ko-KR", name = "Korean (Korea)" },
    { code = "ja-JP", name = "Japanese (Japan)" },
    { code = "en-US", name = "English (United States)" },
]
"#;
        let setting: Setting = toml::from_str(toml).unwrap();
        assert_eq!(
            setting.languages,
            vec![
                Language::new("ko-KR", "Korean (Korea)"),
                Language::new("ja-JP", "Japanese (Japan)"),
                Language::new("en-US", "English (United States)"),
            ]
        );
    }

    #[test]
    fn languages_default_when_unset() {
        let toml = r#""#;
        let setting: Setting = toml::from_str(toml).unwrap();
        assert_eq!(setting.languages.len(), 10);
        assert_eq!(
            setting.languages.first().unwrap(),
            &Language::new("ja-JP", "Japanese (Japan)")
        );
        assert_eq!(
            setting.languages.last().unwrap(),
            &Language::new("en-IN", "English (India)")
        );
    }
}
