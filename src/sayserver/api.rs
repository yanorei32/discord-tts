use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct TtsRequest {
    pub voice_id: String,
    pub text: String,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct Voice {
    pub locale_identifier: String,
    pub name: String,
    pub demo_text: String,
    pub id: String,
}
