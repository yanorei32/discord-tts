use std::borrow::Cow;

pub mod api {
    use base64::{engine::general_purpose::STANDARD as base64_engine, Engine as _};
    use serde::{de, Deserialize, Serialize};

    #[derive(Debug)]
    pub struct DecodedBinary {
        pub bin: Vec<u8>,
    }

    impl<'de> Deserialize<'de> for DecodedBinary {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            let s: &str = de::Deserialize::deserialize(deserializer)?;
            Ok(DecodedBinary {
                bin: base64_engine.decode(s).expect("failed to decode portrait"),
            })
        }
    }

    structstruck::strike! {
        #[derive(Deserialize, Debug)]
        pub struct Speaker {
            pub name: String,
            pub speaker_uuid: String,
            pub styles: Vec<
                #[derive(Deserialize, Debug)]
                pub struct Style {
                    pub name: String,
                    pub id: u32,
                },
            >,
        }
    }

    structstruck::strike! {
        #[derive(Deserialize, Debug)]
        pub struct SpeakerInfo {
            pub policy: String,
            pub style_infos: Vec<
                #[derive(Deserialize, Debug)]
                pub struct StyleInfo {
                    pub id: u32,
                    pub icon: DecodedBinary,
                    pub voice_samples: Vec<DecodedBinary>,
                }
            >,
        }
    }

    #[derive(Debug, Deserialize, Serialize)]
    pub struct Mora {
        pub text: String,
        pub consonant: Option<String>,
        pub consonant_length: Option<f32>,
        pub vowel: String,
        pub vowel_length: f32,
        pub pitch: f32,
    }

    structstruck::strike! {
        #[strikethrough[derive(Debug, Deserialize, Serialize)]]
        #[serde(rename_all = "camelCase")]
        pub struct AudioQuery {
            #[serde(rename = "accent_phrases")]
            pub accent_phrases: Vec<pub struct {
                #![serde(rename_all = "snake_case")]
                pub moras: Vec<Mora>,
                pub accent: u32,
                pub pause_mora: Option<Mora>,
                pub is_interrogative: bool,
            }>,
            pub speed_scale: f32,
            pub pitch_scale: f32,
            pub intonation_scale: f32,
            pub volume_scale: f32,
            pub pre_phoneme_length: f32,
            pub post_phoneme_length: f32,
            pub output_sampling_rate: u32,
            pub output_stereo: bool,
            pub kana: String,
        }
    }
}

pub type SpeakerId = u32;

structstruck::strike! {
    #[derive(Debug)]
    pub struct Speaker<'a> {
        pub name: String,
        pub policy: String,
        pub styles: Vec<
            #[derive(Debug)]
            pub struct SpeakerStyle<'a> {
                pub name: String,
                pub id: SpeakerId,
                pub icon: Cow<'a, [u8]>,
                pub voice_samples: Vec<Cow<'a, [u8]>>,
            }
        >,
    }
}

#[derive(Debug)]
pub struct SpeakerStyleView<'a> {
    pub speaker_i: usize,
    pub speaker_name: &'a str,
    pub speaker_policy: &'a str,
    pub style_i: usize,
    pub style_name: &'a str,
    pub style_id: SpeakerId,
    pub style_icon: Cow<'a, [u8]>,
    pub style_voice_samples: &'a Vec<Cow<'a, [u8]>>,
}
