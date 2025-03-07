use std::borrow::Cow;

pub mod api {
    use base64::{engine::general_purpose::STANDARD as base64_engine, Engine as _};
    use serde::{de, Deserialize};

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
                    #[allow(dead_code)]
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
    #[allow(dead_code)]
    pub style_id: SpeakerId,
    pub style_icon: Cow<'a, [u8]>,
    #[allow(dead_code)]
    pub style_voice_samples: &'a Vec<Cow<'a, [u8]>>,
}
