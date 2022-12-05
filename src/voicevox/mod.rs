use std::borrow::Cow;
use std::str::FromStr;
use std::sync::Mutex;

use crate::CONFIG;
use once_cell::sync::Lazy;
use reqwest::header::CONTENT_TYPE;
use reqwest::Client;
use uuid::Uuid;

pub mod model;

static SPEAKERS: Lazy<Mutex<Vec<model::Speaker>>> = Lazy::new(|| Mutex::new(Vec::new()));

pub async fn load_speaker_info() {
    let config = CONFIG.get().unwrap();
    let client = Client::new();

    let api_speakers: Vec<model::ApiSpeakers> = client
        .get(format!("{}/speakers", config.voicevox_host))
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
            .get(format!("{}/speaker_info", config.voicevox_host))
            .query(&[("speaker_uuid", &uuid)])
            .header(CONTENT_TYPE, "application/json")
            .send()
            .await
            .unwrap_or_else(|_| panic!("Failed to get speaker information of {}", uuid))
            .json()
            .await
            .expect("JSON was not well-formatted");

        let styles = info.style_infos.into_iter().map(|style_info| {
            let samples = style_info.voice_samples.into_iter()
                .map(base64::decode)
                .map(|res| res.expect("Failed to decode sample"))
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
                icon: Cow::from(base64::decode(style_info.icon).expect("Failed to decode icon")),
                samples,
            }
        }).collect();

        let speaker = model::Speaker {
            name: api_speaker.name,
            uuid: Uuid::from_str(uuid.as_str()).expect("Failed to parse UUID from str"),
            policy: info.policy,
            portrait: Cow::from(base64::decode(info.portrait).expect("Failed to decode portrait")),
            styles,
        };

        SPEAKERS.lock().expect("Failed to lock").push(speaker);
    }
}

pub fn get_speakers<'a>() -> Vec<model::Speaker<'a>> {
    SPEAKERS.lock().unwrap().to_vec()
}
