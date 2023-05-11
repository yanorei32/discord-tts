use serde::Deserialize;
use once_cell::sync::Lazy;

pub static CONFIG: Lazy<Config> = Lazy::new(|| {
    envy::from_env().expect("Failed to load Environment variable")
});

#[derive(Deserialize, Debug)]
pub struct Config {
    pub voicevox_host: String,
    pub discord_token: String,
    pub state_path: String,

    #[serde(default = "default_tmp_path")]
    pub tmp_path: String,
}

fn default_tmp_path() -> String {
    "/tmp/".to_string()
}
