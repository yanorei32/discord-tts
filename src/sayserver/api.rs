use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct TtsRequest {
    pub name: String,
    pub text: String,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct Character {
    pub lang: String,
    pub name: String,
    pub sample: String,
}
