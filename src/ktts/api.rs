use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct TtsRequest {
    pub text: String,
}
