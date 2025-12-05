use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Voice {
    pub id: String,
    pub display_name: String,
    pub language: String,
    pub description: String,
    pub gender: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TtsRequest {
    pub text: String,
    pub voice_id: String,
    pub audio_volume: f32,
}
