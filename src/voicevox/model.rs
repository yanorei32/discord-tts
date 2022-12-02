use serde::Deserialize;
use std::borrow::Cow;
use uuid::Uuid;

#[derive(Deserialize, Debug)]
pub struct ApiSpeakers {
    pub name: String,
    pub speaker_uuid: String,
    pub styles: Vec<ApiSpeakersStyles>,
}

#[derive(Deserialize, Debug)]
pub struct ApiSpeakersStyles {
    pub name: String,
    pub id: u32,
}

#[derive(Deserialize, Debug)]
pub struct ApiSpeakerInfo {
    pub policy: String,
    pub portrait: String,
    pub style_infos: Vec<ApiSpeakerInfoStyleInfos>,
}

#[derive(Deserialize, Debug)]
pub struct ApiSpeakerInfoStyleInfos {
    pub id: u32,
    pub icon: String,
    pub voice_samples: Vec<String>,
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
