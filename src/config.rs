use std::path::PathBuf;

use once_cell::sync::Lazy;
use serde::Deserialize;

pub static CONFIG: Lazy<Config> =
    Lazy::new(|| envy::from_env().expect("Failed to load Environment variable"));

#[derive(Deserialize, Debug)]
pub struct Config {
    pub voicevox_host: String,
    pub discord_token: String,
    pub persistent_path: PathBuf,

    #[serde(default = "default_tmp_path")]
    pub tmp_path: PathBuf,
}

fn default_tmp_path() -> PathBuf {
    PathBuf::from("/tmp/")
}
