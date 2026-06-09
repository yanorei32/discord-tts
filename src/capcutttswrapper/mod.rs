use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::{
    Url,
    header::{HeaderMap, HeaderName},
};
use serde::Deserialize;
use tap::Tap;

use crate::tts::{CharacterView, StyleView, TtsService};

mod api;

fn default_master_volume() -> f32 {
    1.0
}

fn default_headers() -> HashMap<String, String> {
    HashMap::new()
}

#[derive(Deserialize, Debug, Clone)]
pub struct Setting {
    pub url: reqwest::Url,
    #[serde(default = "default_headers")]
    pub headers: HashMap<String, String>,
    #[serde(default = "default_master_volume")]
    pub master_volume: f32,
}

#[derive(Debug, Clone)]
pub struct CapCutTTSWrapper {
    inner: Arc<CapCutTTSWrapperInner>,
}

#[derive(Debug)]
struct CapCutTTSWrapperInner {
    host: Url,
    client: reqwest::Client,
    master_volume: f32,
}

#[async_trait]
impl TtsService for CapCutTTSWrapper {
    async fn tts(&self, style_id: &str, text: &str) -> Result<Vec<u8>> {
        let url = self.inner.host.clone().tap_mut(|u| {
            u.path_segments_mut().unwrap().push("v2").push("synthesize");
        });

        let resp = self
            .inner
            .client
            .post(url)
            .json(&api::Request {
                text: text.to_string(),
                speaker: style_id.to_string(),
                method: "buffer".to_string(),
            })
            .send()
            .await
            .context("Failed to post /v2/synthesize (send)")?
            .error_for_status()
            .context("Failed to post /v2/synthesize (status)")?
            .bytes()
            .await
            .context("Failed to post /v2/synthesize (body)")?;

        crate::tts::convert_mp3_to_wav(resp.to_vec(), self.inner.master_volume)
    }

    async fn styles(&self) -> Result<Vec<CharacterView>> {
        let speakers_uri = self.inner.host.clone().tap_mut(|u| {
            u.path_segments_mut().unwrap().push("v2").push("speakers");
        });

        let speakers: Vec<api::Speaker> = self
            .inner
            .client
            .get(speakers_uri)
            .send()
            .await
            .context("Failed to get /v2/speakers (send)")?
            .error_for_status()
            .context("Failed to get /v2/speakers (status)")?
            .json()
            .await
            .context("Failed to get /v2/speakers (body)")?;

        speakers
            .into_iter()
            .map(|speaker| {
                Ok(CharacterView {
                    name: speaker.name,
                    policy: String::from("CapCut ToS"),
                    styles: vec![StyleView {
                        icon: vec![],
                        id: speaker.id,
                        name: "default".to_string(),
                    }],
                })
            })
            .collect()
    }
}

impl CapCutTTSWrapper {
    pub fn new(setting: &Setting) -> Result<CapCutTTSWrapper> {
        let mut headers = HeaderMap::new();

        for (key, value) in &setting.headers {
            headers.insert(
                HeaderName::from_bytes(key.as_bytes()).context("Invalid HeaderName")?,
                value.parse().context("Invalid HeaderValue")?,
            );
        }

        let client = reqwest::ClientBuilder::new()
            .default_headers(headers)
            .user_agent("discord-tts-capcutttswrapper/0.0.0")
            .build()
            .unwrap();

        let host = setting.url.clone();

        Ok(CapCutTTSWrapper {
            inner: Arc::new(CapCutTTSWrapperInner {
                master_volume: setting.master_volume,
                host,
                client,
            }),
        })
    }
}
