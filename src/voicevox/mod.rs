use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use futures::future;
use json::JsonValue;
use reqwest::{
    header::{HeaderMap, HeaderName, CONTENT_TYPE},
    Url,
};
use serde::Deserialize;
use tap::Tap;

use crate::tts::{CharacterView, StyleView, TtsService};

mod api;

fn default_master_volume() -> f64 {
    1.0
}

#[derive(Deserialize, Debug, Clone)]
pub struct Setting {
    pub url: reqwest::Url,
    pub headers: HashMap<String, String>,
    #[serde(default = "default_master_volume")]
    pub master_volume: f64,
}

#[derive(Debug, Clone)]
pub struct Voicevox {
    inner: Arc<VoicevoxInner>,
}

#[derive(Debug)]
struct VoicevoxInner {
    host: Url,
    client: reqwest::Client,
    master_volume: f64,
}

#[async_trait]
impl TtsService for Voicevox {
    async fn tts(&self, style_id: &str, text: &str) -> Result<Vec<u8>> {
        let url = self.inner.host.clone().tap_mut(|u| {
            u.path_segments_mut().unwrap().push("audio_query");
            u.query_pairs_mut()
                .clear()
                .append_pair("text", text)
                .append_pair("speaker", style_id);
        });

        let resp = self
            .inner
            .client
            .post(url)
            .send()
            .await
            .context("Failed to post /audio_query (send)")?;

        let query_text = resp
            .error_for_status()
            .context("Failed to post /audio_query (status)")?
            .text()
            .await
            .context("Failed to post /audio_query (text)")?;

        let url = self.inner.host.clone().tap_mut(|u| {
            u.path_segments_mut().unwrap().push("synthesis");
            u.query_pairs_mut()
                .clear()
                .append_pair("speaker", style_id);
        });

        let query_text = match json::parse(&query_text).context("Faield to parse query")? {
            JsonValue::Object(mut obj) => {
                obj.insert(
                    "volumeScale",
                    JsonValue::Number(self.inner.master_volume.into()),
                );
                json::stringify(obj)
            }
            _ => anyhow::bail!("Non-object JSON is coming"),
        };

        let resp = self
            .inner
            .client
            .post(url)
            .header(CONTENT_TYPE, "application/json")
            .body(query_text)
            .send()
            .await
            .context("Failed to post /synthesis (send)")?
            .error_for_status()
            .context("Failed to post /synthesis (status)")?;

        let bin = resp
            .bytes()
            .await
            .context("Failed to post /synthesis (body)")?;

        Ok(bin.to_vec())
    }

    async fn styles(&self) -> Result<Vec<CharacterView>> {
        let speakers_uri = self.inner.host.clone().tap_mut(|u| {
            u.path_segments_mut().unwrap().push("speakers");
        });

        let speakers: Vec<api::Speaker> = self
            .inner
            .client
            .get(speakers_uri)
            .send()
            .await
            .context("Failed to get /speakers (send)")?
            .error_for_status()
            .context("Failed to get /speakers (status)")?
            .json()
            .await
            .context("Failed to get /speakers (body)")?;

        let speaker_infos: Vec<_> = speakers
            .iter()
            .map(|s| {
                let url = self.inner.host.clone().tap_mut(|u| {
                    u.path_segments_mut().unwrap().push("speaker_info");
                    u.query_pairs_mut()
                        .clear()
                        .append_pair("speaker_uuid", &s.speaker_uuid);
                });

                let client = self.inner.client.clone();

                async move {
                    client
                        .get(url)
                        .send()
                        .await
                        .context("Failed to get /speaker_info (send)")?
                        .error_for_status()
                        .context("Failed to get /speaker_info (status)")?
                        .json::<api::SpeakerInfo>()
                        .await
                        .context("Failed to get /speaker_info (body)")
                }
            })
            .collect();

        let speaker_infos: Vec<_> = future::join_all(speaker_infos).await;
        let speaker_infos: Result<Vec<_>> = speaker_infos.into_iter().collect();
        let speaker_infos = speaker_infos?;

        speakers
            .into_iter()
            .zip(speaker_infos)
            .map(|(speaker, speaker_info)| {
                let speaker_styles: Vec<StyleView> = speaker
                    .styles
                    .into_iter()
                    .zip(speaker_info.style_infos)
                    .map(|(style, style_info)| {
                        assert_eq!(style.id, style_info.id);
                        StyleView {
                            icon: style_info.icon.bin,
                            id: format!("{}", style_info.id),
                            name: style.name,
                        }
                    })
                    .collect();

                let policy = match &speaker_info.policy {
                    policy if policy.starts_with("# Aivis Common Model License (ACML) 1.0\n") =>
                        "この音声は [Aivis Common Model License (ACML) 1.0](https://github.com/Aivis-Project/ACML/blob/master/ACML-1.0.md) により提供されています。".to_string(),
                    policy if policy.starts_with("# Aivis Common Model License (ACML) - Non Commercial 1.0\n") =>
                        "この音声は [Aivis Common Model License (ACML) - Non Commercial 1.0](https://github.com/Aivis-Project/ACML/blob/master/ACML-NC-1.0.md) により提供されています。".to_string(),
                    policy => policy.chars().take(512).collect::<String>(),
                };

                Ok(CharacterView {
                    name: speaker.name,
                    policy,
                    styles: speaker_styles,
                })
            })
            .collect()
    }
}

impl Voicevox {
    pub fn new(setting: &Setting) -> Result<Voicevox> {
        let mut headers = HeaderMap::new();

        for (key, value) in &setting.headers {
            headers.insert(
                HeaderName::from_bytes(key.as_bytes()).context("Invalid HeaderName")?,
                value.parse().context("Invalid HeaderValue")?,
            );
        }

        let client = reqwest::ClientBuilder::new()
            .default_headers(headers)
            .user_agent("discord-tts-voicevox/0.0.0")
            .build()
            .unwrap();

        let host = setting.url.clone();

        Ok(Voicevox {
            inner: Arc::new(VoicevoxInner {
                master_volume: setting.master_volume,
                host,
                client,
            }),
        })
    }
}
