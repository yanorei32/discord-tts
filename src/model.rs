use std::collections::HashMap;
use serenity::model::id::UserId;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub voicevox_host: String,
    pub discord_token: String,
    pub state_path: String,
    pub tmp_path: String,
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub struct UserSettings {
    pub speaker: Option<u8>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct State {
    pub user_settings: HashMap<UserId, UserSettings>,
}
