use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct TtsRequest {
    pub text: String,
    pub speaker: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct Response {
    pub audio: Option<Audio>,
    pub base_resp: BaseResp,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct Audio {
    pub duration: i32,
    pub data: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct BaseResp {
    pub status_code: i32,
    pub status_message: String,
}
