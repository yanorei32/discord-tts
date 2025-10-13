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
                pub id: i64,
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
                pub icon: DecodedBinary,
                pub id: i64,
            }
        >,
    }
}
