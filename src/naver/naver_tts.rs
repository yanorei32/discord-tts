use anyhow::Result;
use reqwest::Url;
use tracing::subscriber::NoSubscriber;

#[derive(Debug, Clone)]
pub struct NaverVoice {
    pub name: &'static str,
    pub speaker: &'static str,
    pub lang: &'static str,
    #[allow(dead_code)]
    pub gender: &'static str,
}

pub const VOICES: &[NaverVoice] = &[
    NaverVoice {
        name: "Danna",
        speaker: "danna",
        lang: "en",
        gender: "f",
    },
    NaverVoice {
        name: "Matt",
        speaker: "matt",
        lang: "en",
        gender: "m",
    },
    NaverVoice {
        name: "Carmen",
        speaker: "carmen",
        lang: "es",
        gender: "f",
    },
    NaverVoice {
        name: "Jose",
        speaker: "jose",
        lang: "es",
        gender: "m",
    },
    NaverVoice {
        name: "Yuri",
        speaker: "yuri",
        lang: "ja",
        gender: "f",
    },
    NaverVoice {
        name: "Shinji",
        speaker: "shinji",
        lang: "ja",
        gender: "m",
    },
    NaverVoice {
        name: "Kyuri",
        speaker: "kyuri",
        lang: "ko",
        gender: "f",
    },
    NaverVoice {
        name: "Jinho",
        speaker: "jinho",
        lang: "ko",
        gender: "m",
    },
    NaverVoice {
        name: "Meimei",
        speaker: "meimei",
        lang: "zh",
        gender: "f",
    },
    NaverVoice {
        name: "Liangliang",
        speaker: "liangliang",
        lang: "zh",
        gender: "m",
    },
];

fn create_empty_wav() -> Result<Vec<u8>> {
    use std::io::Cursor;

    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 24000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut wav_cursor = Cursor::new(Vec::new());
    let wav_writer = hound::WavWriter::new(&mut wav_cursor, spec)?;
    wav_writer.finalize()?;
    Ok(wav_cursor.into_inner())
}

const NAVER_TTS_MAX_CHARS: usize = 500;

pub async fn get_audio_bytes(
    text: &str,
    _lang: &str,
    speaker: &str,
    speed: i32,
    volume: f32,
) -> Result<Vec<u8>> {
    use reqwest::header::{HeaderMap, HeaderValue};

    let parts = crate::tts::split_long_text(text, NAVER_TTS_MAX_CHARS);
    let mut combined_audio = Vec::new();

    for part in parts {
        let mut url = Url::parse("https://dict.naver.com/api/nvoice")?;

        url.query_pairs_mut()
            .append_pair("service", "dictionary")
            .append_pair("speech_fmt", "mp3")
            .append_pair("text", &part)
            .append_pair("speaker", speaker)
            .append_pair("speed", &speed.to_string());

        let mut headers = HeaderMap::new();
        headers.insert("Referer", HeaderValue::from_static("https://dict.naver.com/"));

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;

        let resp = client
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .bytes()
            .await?;

        if resp.is_empty() {
            continue;
        }
        combined_audio.extend_from_slice(&resp);
    }

    if combined_audio.is_empty() {
        return create_empty_wav();
    }

    tracing::subscriber::with_default(NoSubscriber::new(), || {
        crate::tts::convert_mp3_to_wav(combined_audio, volume)
    })
}
