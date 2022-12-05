use anyhow::{anyhow, Result};
use once_cell::sync::OnceCell;
use serde::Deserialize;

static CONFIG: OnceCell<Config> = OnceCell::new();

#[derive(Deserialize, Debug)]
pub struct Config {
    pub voicevox_host: String,
    pub discord_token: String,
    pub state_path: String,
    pub tmp_path: String,
}

pub fn init() -> Result<()> {
    if CONFIG.set(envy::from_env()?).is_err() {
        return Err(anyhow!("Failed to set CONFIG"));
    }

    Ok(())
}

pub fn get() -> &'static Config {
    CONFIG.get().unwrap()
}
