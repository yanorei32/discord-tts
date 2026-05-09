use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct TtsRequest {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pitch: Option<f32>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct Voice {
    pub id: String,
    pub name: String,
    pub quality: f32,
    pub latency: f32,
    pub requires_network_connection: bool,
}
