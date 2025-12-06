use anyhow::Result;
use futures::{SinkExt, StreamExt};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use ssml::Serialize as SsmlSerialize;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use uuid::Uuid;

const TRUSTED_CLIENT_TOKEN: &str = "6A5AA1D4EAFF4E9FB37E23D68491D6F4";
const SEC_MS_GEC_VERSION: &str = "1-130.0.2849.68";
const WIN_EPOCH: u64 = 11644473600;
const BING_SPEECH_MAX_CHARS: usize = 1000;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Voice {
    pub name: String,
    pub short_name: String,
    pub gender: String,
    pub locale: String,
    pub friendly_name: String,
}

fn generate_sec_ms_gec() -> String {
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");

    let mut ticks = since_the_epoch.as_secs();
    ticks += WIN_EPOCH;
    ticks -= ticks % 300;
    let ticks_100ns = ticks as u128 * 10_000_000;

    let str_to_hash = format!("{}{}", ticks_100ns, TRUSTED_CLIENT_TOKEN);
    let mut hasher = Sha256::new();
    hasher.update(str_to_hash);
    hex::encode(hasher.finalize()).to_uppercase()
}

pub async fn list_voices() -> Result<Vec<Voice>> {
    use reqwest::header::{HeaderMap, HeaderValue};

    let url = format!(
        "https://speech.platform.bing.com/consumer/speech/synthesize/readaloud/voices/list?trustedclienttoken={}&Sec-MS-GEC={}&Sec-MS-GEC-Version={}",
        TRUSTED_CLIENT_TOKEN,
        generate_sec_ms_gec(),
        SEC_MS_GEC_VERSION
    );

    let mut headers = HeaderMap::new();
    headers.insert("Authority", HeaderValue::from_static("speech.platform.bing.com"));
    headers.insert(
        "Sec-CH-UA",
        HeaderValue::from_static(
            "\" Not;A Brand\";v=\"99\", \"Microsoft Edge\";v=\"130\", \"Chromium\";v=\"130\"",
        ),
    );
    headers.insert("Sec-CH-UA-Mobile", HeaderValue::from_static("?0"));
    headers.insert("Accept", HeaderValue::from_static("*/*"));
    headers.insert("Sec-Fetch-Site", HeaderValue::from_static("none"));
    headers.insert("Sec-Fetch-Mode", HeaderValue::from_static("cors"));
    headers.insert("Sec-Fetch-Dest", HeaderValue::from_static("empty"));
    headers.insert(
        "User-Agent",
        HeaderValue::from_static(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36 Edg/130.0.0.0",
        ),
    );
    headers.insert(
        "Accept-Encoding",
        HeaderValue::from_static("gzip, deflate, br"),
    );
    headers.insert(
        "Accept-Language",
        HeaderValue::from_static("en-US,en;q=0.9"),
    );

    let client = reqwest::Client::new();
    let resp = client.get(&url).headers(headers).send().await?;
    Ok(resp.json().await?)
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

pub async fn get_audio_bytes(
    text: &str,
    voice: &str,
    locale: &str,
    volume: f32,
) -> Result<Vec<u8>> {
    let parts = crate::tts::split_long_text(text, BING_SPEECH_MAX_CHARS);

    let futures: Vec<_> = parts
        .into_iter()
        .map(|part| {
            fetch_audio_part(
                part,
                voice.to_string(),
                locale.to_string(),
            )
        })
        .collect();

    let results = futures::future::join_all(futures).await;

    let mut combined_audio = Vec::new();
    for result in results {
        if let Ok(audio) = result {
            combined_audio.extend(audio);
        }
    }

    if combined_audio.is_empty() {
        return create_empty_wav();
    }

    crate::tts::convert_mp3_to_wav(combined_audio, volume)
}

async fn fetch_audio_part(
    part: String,
    voice: String,
    locale: String,
) -> Result<Vec<u8>> {
    let url = Url::parse(&format!(
        "wss://speech.platform.bing.com/consumer/speech/synthesize/readaloud/edge/v1?TrustedClientToken={}&Sec-MS-GEC={}&Sec-MS-GEC-Version={}",
        TRUSTED_CLIENT_TOKEN,
        generate_sec_ms_gec(),
        SEC_MS_GEC_VERSION
    ))?;

    let (mut ws_stream, _) = connect_async(url.to_string()).await?;

    ws_stream
        .send(Message::Text(
            "Content-Type:application/json; charset=utf-8\r\n\
            Path:speech.config\r\n\r\n\
            {\"context\":{\"synthesis\":{\"audio\":{\"outputFormat\":\"audio-24khz-48kbitrate-mono-mp3\"}}}}"
            .to_string().into(),
        ))
        .await?;

    let locale_regex = regex::Regex::new(r"^[a-z]{2}-[A-Z]{2}$").unwrap();
    let locale_option = if locale_regex.is_match(&locale) {
        Some(locale.as_str())
    } else {
        None
    };

    let doc = ssml::speak(locale_option, [ssml::voice(&voice, [part])]);
    let ssml_string = doc.serialize_to_string(&ssml::SerializeOptions::default())?;

    ws_stream
        .send(Message::Text(format!(
            "X-RequestId:{}\r\n\
            Content-Type:application/ssml+xml\r\n\
            Path:ssml\r\n\r\n\
            {}",
            Uuid::new_v4().simple(),
            ssml_string
        ).into()))
        .await?;

    let mut audio_data = Vec::new();

    while let Some(msg) = ws_stream.next().await {
        match msg? {
            Message::Text(text) if text.contains("Path:turn.end") => break,
            Message::Binary(data) if data.len() >= 2 => {
                let header_len = u16::from_be_bytes([data[0], data[1]]) as usize;
                if data.len() >= header_len + 2 {
                    let headers = String::from_utf8_lossy(&data[2..2 + header_len]);
                    if headers.contains("Path:audio") && headers.contains("Content-Type:audio/mpeg")
                    {
                        audio_data.extend_from_slice(&data[2 + header_len..]);
                    }
                }
            }
            _ => {}
        }
    }

    Ok(audio_data)
}
