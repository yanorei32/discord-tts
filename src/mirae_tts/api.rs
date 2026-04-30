use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct G2pRequest {
    pub style: String,
    pub text: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct G2pResponse {
    pub text: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TtsRequest {
    pub text: String,
}
