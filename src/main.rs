#![warn(clippy::pedantic)]

mod commands;
mod db;
mod filter;
mod model;
mod songbird_handler;
mod tts;
mod wavsource;
mod voicevox;
mod voiceroid;

use std::io::Cursor;

use anyhow::Context as _;
use clap::Parser;
use once_cell::sync::OnceCell;
use serenity::{
    all::{ChunkGuildFilter, Guild},
    async_trait,
    client::{Client, Context, EventHandler},
    model::{
        application::{Command, Interaction},
        channel::Message,
        gateway::Ready,
        prelude::GatewayIntents,
    },
};
use songbird::SerenityInit;

use crate::db::PERSISTENT_DB;
use crate::model::TtsServiceConfig;
use crate::tts::TtsServices;
use crate::voiceroid::Voiceroid;
use crate::voicevox::Voicevox;

struct Bot {
    tts_services: TtsServices,
    prefix: String,
}

#[async_trait]
impl EventHandler for Bot {
    async fn ready(&self, ctx: Context, ready: Ready) {
        Command::set_global_commands(
            &ctx.http,
            vec![
                commands::join::register(&self.prefix),
                commands::leave::register(&self.prefix),
                commands::skip::register(&self.prefix),
                commands::speaker::register(&self.prefix),
            ],
        )
        .await
        .unwrap();

        println!("{} is connected!", ready.user.name);
    }

    async fn guild_create(&self, ctx: Context, guild: Guild, _is_new: Option<bool>) {
        ctx.shard
            .chunk_guild(guild.id, None, false, ChunkGuildFilter::None, None);
    }

    async fn message(&self, ctx: Context, msg: Message) {
        let Some(content) = filter::filter(&ctx, &msg) else {
            return;
        };

        let speaker = PERSISTENT_DB
            .get_voice_setting(msg.author.id)
            .unwrap_or(DEFAULT_TTS_STYLE.get().unwrap().clone());

        // Check avialablity
        let speaker = if self
            .tts_services
            .is_available(&speaker.service_id, &speaker.style_id)
            .await
        {
            speaker
        } else {
            DEFAULT_TTS_STYLE.get().unwrap().clone()
        };

        let manager = songbird::get(&ctx)
            .await
            .expect("Songbird is not initialized");

        let handler = manager.get(msg.guild_id.unwrap()).unwrap();

        let wav = match self
            .tts_services
            .tts(&speaker.service_id, &speaker.style_id, &content)
            .await
        {
            Err(e) => {
                msg.reply(
                    &ctx.http,
                    &format!("Error: Failed to synthesise a message {e}"),
                )
                .await
                .unwrap();
                return;
            }
            Ok(v) => v,
        };

        let (source, sample_rate) = wavsource::WavSource::new(&mut Cursor::new(wav));
        handler
            .lock()
            .await
            .enqueue_input(songbird::input::RawAdapter::new(source, sample_rate, 1).into())
            .await;
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        let prefix = &self.prefix;
        match interaction {
            Interaction::Command(command) => match command.data.name.as_str() {
                s if s == format!("{prefix}speaker") => {
                    commands::speaker::run(&ctx, command, &self.tts_services).await;
                }
                s if s == format!("{prefix}join") => commands::join::run(&ctx, command).await,
                s if s == format!("{prefix}leave") => commands::leave::run(&ctx, command).await,
                s if s == format!("{prefix}skip") => commands::skip::run(&ctx, command).await,
                _ => unreachable!("Unknown command: {}", command.data.name),
            },
            Interaction::Component(interaction) => {
                commands::speaker::update(&ctx, interaction, &self.tts_services).await;
            }
            _ => {}
        }
    }
}

static DEFAULT_TTS_STYLE: OnceCell<model::TtsStyle> = OnceCell::new();
static CLI_OPTIONS: OnceCell<model::Cli> = OnceCell::new();

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    CLI_OPTIONS.set(model::Cli::parse()).unwrap();

    let cli = CLI_OPTIONS.get().unwrap();

    let tts_config = model::TtsConfig::new(&cli.tts_config_path).unwrap();

    DEFAULT_TTS_STYLE
        .set(tts_config.default_style.clone())
        .unwrap();

    let tts_services = TtsServices::new();

    for (service_id, service) in &tts_config.tts_services {
        match service {
            TtsServiceConfig::Voiceroid(config) => {
                tts_services
                    .register(
                        service_id,
                        Box::new(
                            Voiceroid::new(config)
                                .await
                                .with_context(|| {
                                    format!("Failed to initialize VOICEROID backend ({service_id})")
                                })
                                .unwrap(),
                        ),
                    )
                    .await
            }
            TtsServiceConfig::Voicevox(config) => {
                tts_services
                    .register(
                        service_id,
                        Box::new(
                            Voicevox::new(config)
                                .with_context(|| {
                                    format!("Failed to initialize VOICEROID backend ({service_id})")
                                })
                                .unwrap(),
                        ),
                    )
                    .await
            }
        }
        .with_context(|| format!("Failed to register service {service_id}"))
        .unwrap();
    }

    let intents = GatewayIntents::GUILDS
        | GatewayIntents::GUILD_VOICE_STATES
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(&cli.discord_token, intents)
        .event_handler(Bot {
            tts_services,
            prefix: cli.command_prefix.clone().unwrap_or_default(),
        })
        .register_songbird()
        .await
        .expect("Failed to create client");

    tokio::spawn(async move {
        let _: Result<_, _> = client
            .start()
            .await
            .map_err(|why| println!("Client ended: {why:?}"));
    });

    tokio::signal::ctrl_c()
        .await
        .expect("Failed to wait Ctrl+C");

    println!("Received Ctrl+C, shutting down.");
}
