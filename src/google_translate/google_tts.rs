use anyhow::Result;
use reqwest::Url;

const GOOGLE_TTS_MAX_CHARS: usize = 200;

pub async fn get_audio_bytes(
    text: &str,
    lang: &str,
    slow: bool,
    host: &Url,
    volume: f32,
) -> Result<Vec<u8>> {
    let parts = crate::tts::split_long_text(text, GOOGLE_TTS_MAX_CHARS);
    let mut combined_audio = Vec::new();

    for part in parts {
        let mut url = host.clone();
        url.path_segments_mut()
            .map_err(|()| anyhow::anyhow!("Cannot be base"))?
            .push("translate_tts");

        url.query_pairs_mut()
            .append_pair("ie", "UTF-8")
            .append_pair("q", &part)
            .append_pair("tl", lang)
            .append_pair("total", "1")
            .append_pair("idx", "0")
            .append_pair("textlen", &part.len().to_string())
            .append_pair("tk", "0")
            .append_pair("client", "tw-ob")
            .append_pair("ttsspeed", if slow { "0" } else { "1" });

        let resp = reqwest::get(url).await?.error_for_status()?.bytes().await?;

        combined_audio.extend_from_slice(&resp);
    }

    crate::tts::convert_mp3_to_wav(combined_audio, volume)
}
