use anyhow::Result;
use reqwest::Url;

#[derive(Debug, Clone)]
pub struct NaverVoice {
    pub name: &'static str,
    pub speaker: &'static str,
    pub lang: &'static str,
    #[allow(dead_code)]
    pub gender: &'static str,
}

pub const VOICES: &[NaverVoice] = &[
    NaverVoice { name: "Danna", speaker: "danna", lang: "en", gender: "f" },
    NaverVoice { name: "Matt", speaker: "matt", lang: "en", gender: "m" },
    NaverVoice { name: "Carmen", speaker: "carmen", lang: "es", gender: "f" },
    NaverVoice { name: "Jose", speaker: "jose", lang: "es", gender: "m" },
    NaverVoice { name: "Yuri", speaker: "yuri", lang: "ja", gender: "f" },
    NaverVoice { name: "Shinji", speaker: "shinji", lang: "ja", gender: "m" },
    NaverVoice { name: "Kyuri", speaker: "kyuri", lang: "ko", gender: "f" },
    NaverVoice { name: "Jinho", speaker: "jinho", lang: "ko", gender: "m" },
    NaverVoice { name: "Meimei", speaker: "meimei", lang: "zh", gender: "f" },
    NaverVoice { name: "Liangliang", speaker: "liangliang", lang: "zh", gender: "m" },
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

pub async fn get_audio_bytes(
    text: &str,
    _lang: &str,
    speaker: &str,
    speed: i32,
    host: &Url,
    volume: f32,
) -> Result<Vec<u8>> {
    use reqwest::header::{HeaderMap, HeaderValue};

    let mut url = host.clone();
    url.path_segments_mut()
        .map_err(|()| anyhow::anyhow!("Cannot be base"))?
        .push("api")
        .push("nvoice");

    url.query_pairs_mut()
        .append_pair("service", "dictionary")
        .append_pair("speech_fmt", "mp3")
        .append_pair("text", text)
        .append_pair("speaker", speaker)
        .append_pair("speed", &speed.to_string());

    let mut headers = HeaderMap::new();
    headers.insert(
        "Referer",
        HeaderValue::from_str(host.as_str())?,
    );

    let client = reqwest::Client::builder()
        .default_headers(headers)
        .build()?;

    let resp = client.get(url).send().await?.error_for_status()?.bytes().await?;

    if resp.is_empty() {
        return create_empty_wav();
    }

    match convert_to_wav(resp.to_vec(), volume) {
        Ok(wav) => Ok(wav),
        Err(_) => create_empty_wav(),
    }
}

fn convert_to_wav(mp3_data: Vec<u8>, gain: f32) -> Result<Vec<u8>> {
    use std::io::Cursor;
    use symphonia::core::audio::{AudioBufferRef, Signal};
    use symphonia::core::codecs::DecoderOptions;
    use symphonia::core::formats::FormatOptions;
    use symphonia::core::io::{MediaSourceStream, MediaSourceStreamOptions};
    use symphonia::core::meta::MetadataOptions;
    use symphonia::core::probe::Hint;

    let mss = MediaSourceStream::new(
        Box::new(Cursor::new(mp3_data)),
        MediaSourceStreamOptions::default(),
    );
    let mut hint = Hint::new();
    hint.with_extension("mp3");

    let probed = symphonia::default::get_probe().format(
        &hint,
        mss,
        &FormatOptions::default(),
        &MetadataOptions::default(),
    )?;

    let mut format = probed.format;
    let track = format
        .default_track()
        .ok_or_else(|| anyhow::anyhow!("No track found"))?;
    let mut decoder =
        symphonia::default::get_codecs().make(&track.codec_params, &DecoderOptions::default())?;

    let track_id = track.id;
    let spec = hound::WavSpec {
        channels: 1,        // Naver TTS is usually mono
        sample_rate: 24000, // Naver TTS is usually 24kHz
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut wav_cursor = Cursor::new(Vec::new());
    let mut wav_writer = hound::WavWriter::new(&mut wav_cursor, spec)?;

    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(symphonia::core::errors::Error::IoError(_)) => break, // End of stream
            Err(e) => return Err(e.into()),
        };

        if packet.track_id() != track_id {
            continue;
        }

        match decoder.decode(&packet) {
            Ok(decoded_packet) => match decoded_packet {
                AudioBufferRef::F32(buf) => {
                    for &sample in buf.chan(0) {
                        let sample = sample * gain * f32::from(i16::MAX);
                        let sample = sample.min(f32::from(i16::MAX)).max(f32::from(i16::MIN));

                        #[allow(clippy::cast_possible_truncation)]
                        wav_writer.write_sample(sample as i16)?;
                    }
                }
                AudioBufferRef::S16(buf) => {
                    for &sample in buf.chan(0) {
                        let sample = f32::from(sample) * gain;
                        let sample = sample.min(f32::from(i16::MAX)).max(f32::from(i16::MIN));

                        wav_writer.write_sample(sample)?;
                    }
                }
                _ => anyhow::bail!("Unsupported audio format"),
            },
            Err(symphonia::core::errors::Error::IoError(_)) => break,
            Err(symphonia::core::errors::Error::DecodeError(_)) => {} // Skip decode errors
            Err(e) => return Err(e.into()),
        }
    }

    wav_writer.finalize()?;
    Ok(wav_cursor.into_inner())
}
