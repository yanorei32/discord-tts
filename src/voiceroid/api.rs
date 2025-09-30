use serde::{Serialize, Deserialize};

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Voice {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub dialect: String,
    pub gender: String,
    pub background_color: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TtsRequest {
    pub is_kansai: bool,
    pub voice_id: String,
    pub text: String,
}
