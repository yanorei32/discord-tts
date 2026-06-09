use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct TtsRequest {
    pub text: String,
    pub voice_id: String,
    pub speed: f32,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct Voice {
    pub voice_id: String,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct Voices {
    pub voices: Vec<Voice>,
}
