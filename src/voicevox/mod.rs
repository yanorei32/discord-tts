use std::borrow::Cow;
use std::sync::Arc;
use std::sync::Mutex;

use base64::{engine::general_purpose::STANDARD as base64_engine, Engine as _};
use bytes::Bytes;
use once_cell::sync::Lazy;
use reqwest::{header::CONTENT_TYPE, Url};
use tap::prelude::*;

use crate::config::CONFIG;

pub mod model;

static SPEAKERS: Lazy<Mutex<Vec<model::Speaker>>> = Lazy::new(|| Mutex::new(Vec::new()));

#[derive(Debug, Clone)]
pub struct Client {
    inner: Arc<InnerClient>,
}

#[derive(Debug)]
pub struct InnerClient {
    host: Url,
    client: reqwest::Client,
}

type SpeakerId = u64;

impl Client {
    pub fn new(host: Url, client: reqwest::Client) -> Self {
        Self {
            inner: Arc::new(InnerClient { host, client }),
        }
    }

    pub async fn tts(&self, text: &str, speaker_id: SpeakerId) -> Bytes {
        let url = self.inner.host.clone().tap_mut(|u| {
            u.path_segments_mut().unwrap().push("audio_query");
            u.query_pairs_mut()
                .clear()
                .append_pair("text", text)
                .append_pair("speaker", &speaker_id.to_string());
        });

        let resp = self.inner.client.post(url).send().await.unwrap();
        let query_text = resp.text().await.unwrap();

        let url = self.inner.host.clone().tap_mut(|u| {
            u.path_segments_mut().unwrap().push("synthesis");
            u.query_pairs_mut()
                .clear()
                .append_pair("speaker", &speaker_id.to_string());
        });

        let resp = self
            .inner
            .client
            .post(url)
            .header(CONTENT_TYPE, "application/json")
            .body(query_text)
            .send()
            .await
            .unwrap();

        resp.bytes().await.unwrap()
    }
}

pub async fn load_speaker_info() {
    let client = reqwest::Client::new();

    let api_speakers: Vec<model::ApiSpeakers> = client
        .get(format!("{}/speakers", CONFIG.voicevox_host))
        .header(CONTENT_TYPE, "application/json")
        .send()
        .await
        .expect("Failed to get speakers")
        .json()
        .await
        .expect("JSON was not well-formatted");

    for api_speaker in api_speakers {
        let uuid = api_speaker.speaker_uuid;

        let info: model::ApiSpeakerInfo = client
            .get(format!("{}/speaker_info", CONFIG.voicevox_host))
            .query(&[("speaker_uuid", &uuid)])
            .header(CONTENT_TYPE, "application/json")
            .send()
            .await
            .unwrap_or_else(|_| panic!("Failed to get speaker information of {uuid}"))
            .json()
            .await
            .expect("JSON was not well-formatted");

        let styles = info
            .style_infos
            .into_iter()
            .map(|style_info| {
                let samples = style_info
                    .voice_samples
                    .into_iter()
                    .map(|s| base64_engine.decode(s).expect("Failed to decode sample"))
                    .map(Cow::from)
                    .collect();

                model::SpeakerStyle {
                    name: api_speaker
                        .styles
                        .iter()
                        .find(|api_style| api_style.id == style_info.id)
                        .expect("Style not found")
                        .name
                        .clone(),
                    id: style_info.id,
                    icon: Cow::from(
                        base64_engine
                            .decode(style_info.icon)
                            .expect("Failed to decode icon"),
                    ),
                    samples,
                }
            })
            .collect();

        let speaker = model::Speaker {
            name: api_speaker.name,
            policy: info.policy,
            portrait: Cow::from(
                base64_engine
                    .decode(info.portrait)
                    .expect("Failed to decode portrait"),
            ),
            styles,
        };

        SPEAKERS.lock().expect("Failed to lock").push(speaker);
    }
}

pub fn get_speakers<'a>() -> Vec<model::Speaker<'a>> {
    SPEAKERS.lock().unwrap().to_vec()
}
