use std::io::Cursor;

use anyhow::{Context, Result};
use futures::future::join_all;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE, ORIGIN, REFERER, USER_AGENT};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct CoefontVoice {
    pub name: &'static str,
    pub id: &'static str,
    pub lang: &'static str,
}

pub const VOICES: &[CoefontVoice] = &[
    CoefontVoice {
        name: "森川智之",
        id: "3f84b7b1-30fb-4677-a704-fd136515303e",
        lang: "ja",
    },
    CoefontVoice {
        name: "ひろゆき",
        id: "19d55439-312d-4a1d-a27b-28f0f31bedc5",
        lang: "ja",
    },
    CoefontVoice {
        name: "女性アナウンサー",
        id: "76e2ba06-b23a-4bbe-8148-e30ede9001b9",
        lang: "ja",
    },
    CoefontVoice {
        name: "男性ナレーター",
        id: "82c4fcf5-d0ee-4fe9-9b0d-89a65d04f290",
        lang: "ja",
    },
    CoefontVoice {
        name: "森川智之",
        id: "2ad85567-b98d-433b-ac8b-eecbb409d1c9",
        lang: "en",
    },
    CoefontVoice {
        name: "ひろゆき",
        id: "d757e390-a61a-4718-a4a5-5cb4e0b3cbb0",
        lang: "en",
    },
    CoefontVoice {
        name: "女性アナウンサー",
        id: "66e41ffd-693c-422f-b4b7-6f9e85986de1",
        lang: "en",
    },
    CoefontVoice {
        name: "男性ナレーター",
        id: "ac121510-eea6-4177-ba8d-4e3b9aba5359",
        lang: "en",
    },
    CoefontVoice {
        name: "森川智之",
        id: "702b9444-30b1-4f7e-874f-2e48fe4c49fb",
        lang: "zh",
    },
    CoefontVoice {
        name: "ひろゆき",
        id: "86e73a8d-b22a-4c28-be45-140435dd83c8",
        lang: "zh",
    },
    CoefontVoice {
        name: "女性アナウンサー",
        id: "bbfe3aa9-8396-41cd-90dc-dbbddf918f0c",
        lang: "zh",
    },
    CoefontVoice {
        name: "男性ナレーター",
        id: "8371f2dd-53d9-4f24-a486-c1a666e49e68",
        lang: "zh",
    },
];

#[derive(Serialize)]
struct TtsPayload {
    text: String,
    variant: String,
}

#[derive(Deserialize, Debug)]
struct TtsResponse {
    location: Option<String>,
    url: Option<String>,
}

const COEFONT_MAX_CHARS: usize = 30;

pub async fn get_audio_bytes(text: &str, voice_id: &str, volume: f32) -> Result<Vec<u8>> {
    let parts = crate::tts::split_long_text(text, COEFONT_MAX_CHARS);

    let futures: Vec<_> = parts
        .into_iter()
        .map(|part| fetch_audio_part(part, voice_id.to_string()))
        .collect();

    let results = join_all(futures).await;

    let mut wav_parts = Vec::new();
    for result in results {
        wav_parts.push(result?);
    }

    if wav_parts.is_empty() {
        return create_empty_wav();
    }

    let combined_wav = concatenate_wav_files(wav_parts)?;
    apply_volume_to_wav(combined_wav, volume)
}

fn concatenate_wav_files(wav_files: Vec<Vec<u8>>) -> Result<Vec<u8>> {
    use std::io::Cursor;

    if wav_files.is_empty() {
        return create_empty_wav();
    }

    if wav_files.len() == 1 {
        return Ok(wav_files.into_iter().next().unwrap());
    }

    let first_reader = hound::WavReader::new(Cursor::new(&wav_files[0]))?;
    let spec = first_reader.spec();

    let mut all_samples = Vec::new();
    
    for wav_data in wav_files {
        let mut reader = hound::WavReader::new(Cursor::new(&wav_data))?;
        let samples: Vec<i16> = reader
            .samples::<i16>()
            .collect::<Result<Vec<_>, _>>()?;
        all_samples.extend(samples);
    }

    let mut wav_cursor = Cursor::new(Vec::new());
    let mut wav_writer = hound::WavWriter::new(&mut wav_cursor, spec)?;
    for sample in all_samples {
        wav_writer.write_sample(sample)?;
    }
    wav_writer.finalize()?;
    Ok(wav_cursor.into_inner())
}

fn apply_volume_to_wav(wav_data: Vec<u8>, volume: f32) -> Result<Vec<u8>> {
    if (volume - 1.0).abs() < 0.01 {
        return Ok(wav_data);
    }

    let mut reader = hound::WavReader::new(Cursor::new(&wav_data))?;
    let spec = reader.spec();

    #[allow(clippy::cast_possible_truncation)]
    let samples: Vec<i16> = reader
        .samples::<i16>()
        .map(|s| s.unwrap_or(0))
        .map(|s| (f32::from(s) * volume).clamp(f32::from(i16::MIN), f32::from(i16::MAX)) as i16)
        .collect();

    let mut wav_cursor = Cursor::new(Vec::new());
    let mut wav_writer = hound::WavWriter::new(&mut wav_cursor, spec)?;
    for sample in samples {
        wav_writer.write_sample(sample)?;
    }
    wav_writer.finalize()?;
    Ok(wav_cursor.into_inner())
}

async fn fetch_audio_part(text: String, voice_id: String) -> Result<Vec<u8>> {
    let url = format!("https://backend.coefont.cloud/coefonts/{voice_id}/try");

    let payload = TtsPayload {
        text,
        variant: "lp-tts".to_string(),
    };

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(USER_AGENT, HeaderValue::from_static("Mozilla/5.0"));
    headers.insert(ORIGIN, HeaderValue::from_static("https://coefont.cloud"));
    headers.insert(REFERER, HeaderValue::from_static("https://coefont.cloud/"));

    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .headers(headers)
        .json(&payload)
        .send()
        .await
        .context("Failed to send request")?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        anyhow::bail!("API request failed with status {status}: {error_text}");
    }

    let tts_response: TtsResponse = response
        .json()
        .await
        .context("Failed to parse JSON response")?;

    let audio_url = tts_response
        .location
        .or(tts_response.url)
        .context("No audio URL found in response")?;

    let audio_bytes = client
        .get(&audio_url)
        .send()
        .await
        .context("Failed to download audio")?
        .bytes()
        .await
        .context("Failed to get audio bytes")?;

    Ok(audio_bytes.to_vec())
}

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
