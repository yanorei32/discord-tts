use std::path::PathBuf;
use clap::Parser;

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

    #[clap(env, long)]
    pub auto_leave_when_alone: bool,
}
