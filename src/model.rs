use std::collections::HashMap;
use std::path::{Path, PathBuf};

// use once_cell::sync::Lazy;
use anyhow::{Context, Result};
use clap::Parser;
use serde::{Deserialize, Serialize};

// pub static CONFIG: Lazy<Config> =
//     Lazy::new(|| envy::from_env().expect("Failed to load Environment variable"));

#[derive(Parser, Debug)]
pub struct Cli {
    #[clap(env, long, default_value = "/etc/discord-tts.tts.toml")]
    pub tts_config_path: PathBuf,

    #[clap(env, long)]
    pub command_prefix: Option<String>,

    #[clap(env, long)]
    pub discord_token: String,

    #[clap(env, long, default_value = "/var/discordtts/state.json")]
    pub persistent_path: PathBuf,
}

#[derive(Deserialize, Debug)]
pub enum TtsServiceConfig {
    Voiceroid(crate::voiceroid::Setting),
    Voicevox(crate::voicevox::Setting),
}

#[derive(Deserialize, Debug)]
pub struct TtsConfig {
    pub default_style: TtsStyle,
    pub tts_services: HashMap<String, TtsServiceConfig>,
}

impl TtsConfig {
    pub fn new(path: &Path) -> Result<Self> {
        let s = std::fs::read_to_string(path).context("Failed to read TtsConfig")?;
        toml::from_str(&s).context("Failed to parse TtsConfig")
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TtsStyle {
    pub service_id: String,
    pub style_id: String,
}
