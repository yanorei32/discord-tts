use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct Speaker {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct Request {
    pub text: String,
    pub speaker: String,
    pub method: String,
}
