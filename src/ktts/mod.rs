use std::collections::HashMap;
use std::io::Cursor;
use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use hound::{WavReader, WavWriter};
use reqwest::header::{HeaderMap, HeaderName};
use serde::Deserialize;
use tap::Tap;

use crate::tts::{CharacterView, StyleView, TtsService};

mod api;

fn default_master_volume() -> f32 {
    1.0
}

#[derive(Deserialize, Debug, Clone)]
pub struct Setting {
    pub url: reqwest::Url,
    pub headers: HashMap<String, String>,
    #[serde(default = "default_master_volume")]
    pub master_volume: f32,
}

#[derive(Debug)]
struct KTTSInner {
    client: reqwest::Client,
    url: reqwest::Url,
    master_volume: f32,
}

fn gain(buffer: &[u8], gain: f32) -> Result<Vec<u8>> {
    let mut in_wav =
        WavReader::new(Cursor::new(buffer)).with_context(|| "Failed to read as wav file")?;
    let spec = in_wav.spec();
    let mut buffer = Cursor::new(vec![]);
    let mut out_wav = WavWriter::new(&mut buffer, spec).with_context(|| "Failed to create wav")?;

    for sample in in_wav.samples::<i16>().map(|s| s.unwrap()) {
        out_wav
            .write_sample::<i16>((sample as f32 * gain) as i16)
            .with_context(|| "Failed to write sample")?;
    }

    out_wav.finalize().with_context(|| "Failed to finalize wav file")?;

    Ok(buffer.into_inner())
}

#[derive(Clone, Debug)]
pub struct KTTS {
    inner: Arc<KTTSInner>,
}

impl KTTS {
    pub fn new(setting: &Setting) -> Result<Self> {
        let mut headers = HeaderMap::new();

        for (key, value) in &setting.headers {
            headers.insert(
                HeaderName::from_bytes(key.as_bytes()).context("Invalid HeaderName")?,
                value.parse().context("Invalid HeaderValue")?,
            );
        }

        let client = reqwest::ClientBuilder::new()
            .default_headers(headers)
            .user_agent("discord-tts-voiceroid/0.0.0")
            .build()
            .unwrap();

        Ok(KTTS {
            inner: Arc::new(KTTSInner {
                url: setting.url.clone(),
                master_volume: setting.master_volume,
                client,
            }),
        })
    }
}

#[async_trait]
impl TtsService for KTTS {
    async fn tts(&self, _style_id: &str, text: &str) -> Result<Vec<u8>> {
        let api_tts = self.inner.url.clone().tap_mut(|u| {
            u.path_segments_mut().unwrap().push("api").push("tts");
        });

        let query = api::TtsRequest {
            text: text.to_string(),
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

        Ok(gain(&resp.to_vec(), self.inner.master_volume)?)
    }

    async fn styles(&self) -> Result<Vec<CharacterView>> {
        Ok(vec![CharacterView {
            name: "Default".to_string(),
            policy: "조선어음성합성프로그람 《청봉》 3.2 by RedStar 3.0".to_string(),
            styles: vec![StyleView {
                icon: vec![],
                name: "Default".to_string(),
                id: "Default".to_string(),
            }],
        }])
    }
}
