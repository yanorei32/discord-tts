use std::borrow::Cow;
use std::str::FromStr;
use std::sync::Mutex;
use once_cell::sync::Lazy;
use reqwest::Client;
use reqwest::header::CONTENT_TYPE;
use serde::Deserialize;
use uuid::Uuid;
use crate::CONFIG;

#[derive(Deserialize, Debug)]
struct ApiSpeakers {
    name: String,
    speaker_uuid: String,
    styles: Vec<ApiSpeakersStyles>,
}

#[derive(Deserialize, Debug)]
struct ApiSpeakersStyles {
    name: String,
    id: u32,
}

#[derive(Deserialize, Debug)]
struct ApiSpeakerInfo {
    policy: String,
    portrait: String,
    style_infos: Vec<ApiSpeakerInfoStyleInfos>,
}

#[derive(Deserialize, Debug)]
struct ApiSpeakerInfoStyleInfos {
    id: u32,
    icon: String,
    voice_samples: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct Speaker<'a> {
    pub name: String,
    pub uuid: Uuid,
    pub policy: String,
    pub portrait: Cow<'a, [u8]>,
    pub styles: Vec<SpeakerStyle<'a>>,
}

#[derive(Clone, Debug)]
pub struct SpeakerStyle<'a> {
    pub name: String,
    pub id: u32,
    pub icon: Cow<'a, [u8]>,
    pub samples: Vec<Cow<'a, [u8]>>,
}

static SPEAKERS: Lazy<Mutex<Vec<Speaker>>> = Lazy::new(|| Mutex::new(Vec::new()));

pub async fn load_speaker_info() {
    let config = CONFIG.get().unwrap();
    let client = Client::new();

    let api_speakers: Vec<ApiSpeakers> = client
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

        let info: ApiSpeakerInfo = client
            .get(format!("{}/speaker_info", config.voicevox_host))
            .query(&[("speaker_uuid", &uuid)])
            .header(CONTENT_TYPE, "application/json")
            .send()
            .await
            .expect(format!("Failed to get speaker information of {}", uuid).as_str())
            .json()
            .await
            .expect("JSON was not well-formatted");

        let mut styles = Vec::new();
        for style_info in info.style_infos {
            let mut samples = Vec::new();
            for sample in style_info.voice_samples {
                let sample = base64::decode(sample).expect("Failed to decode sample");
                samples.push(Cow::from(sample));
            }

            let style = SpeakerStyle {
                name: api_speaker.styles.iter()
                    .find(|api_style| api_style.id == style_info.id)
                    .expect("Style not found")
                    .name.to_owned(),
                id: style_info.id,
                icon: Cow::from(
                    base64::decode(style_info.icon)
                        .expect("Failed to decode icon")
                ),
                samples,
            };

            styles.push(style);
        }

        let speaker = Speaker {
            name: api_speaker.name,
            uuid: Uuid::from_str(uuid.as_str()).expect("Failed to parse UUID from str"),
            policy: info.policy,
            portrait: Cow::from(
                base64::decode(info.portrait)
                    .expect("Failed to decode portrait"),
            ),
            styles
        };

        SPEAKERS.lock().expect("Failed to lock").push(speaker)
    }
}

pub fn get_speakers<'a>() -> Vec<Speaker<'a>> {
    SPEAKERS.lock().unwrap().to_vec()
}
