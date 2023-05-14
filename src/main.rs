#![warn(clippy::pedantic)]

mod commands;
mod config;
mod db;
mod filter;
mod interactive_component;
mod model;
mod songbird_handler;
mod voicevox;

use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Cursor};
use std::path::Path;
use std::sync::Mutex;

use once_cell::sync::Lazy;
use reqwest::header::CONTENT_TYPE;
use serenity::{
    async_trait,
    client::{Client, Context, EventHandler},
    model::{
        application::{command::Command, interaction::Interaction},
        channel::Message,
        gateway::Ready,
        prelude::{ChannelId, GatewayIntents, GuildId},
    },
};
use songbird::{ffmpeg, tracks::create_player, Event, SerenityInit, TrackEvent};
use uuid::Uuid;

use crate::config::CONFIG;
use crate::db::STATE_DB;
use crate::model::SpeakerSelector;

static WATCH_CHANNELS: Lazy<Mutex<HashMap<GuildId, ChannelId>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        Command::set_global_application_commands(&ctx.http, |commands| {
            commands
                .create_application_command(commands::join::register)
                .create_application_command(commands::leave::register)
                .create_application_command(commands::skip::register)
                .create_application_command(commands::speaker::register)
        })
        .await
        .unwrap();

        println!("{} is connected!", ready.user.name);
    }

    async fn message(&self, ctx: Context, msg: Message) {
        let Some(content) = filter::filter(&ctx, &msg).await else {
            return;
        };

        let manager = songbird::get(&ctx)
            .await
            .expect("Songbird is not initialized");

        let Some(handler) = manager.get(msg.guild_id.unwrap()) else {
            return;
        };

        if WATCH_CHANNELS
            .lock()
            .unwrap()
            .get(&msg.guild_id.unwrap())
            .map_or(true, |id| id != &msg.channel_id)
        {
            return;
        }

        let speaker = STATE_DB.get_speaker_id(msg.author.id);
        let params = [("text", &content), ("speaker", &speaker.to_string())];
        let client = reqwest::Client::new();
        let query = client
            .post(format!("{}/audio_query", CONFIG.voicevox_host))
            .query(&params)
            .send()
            .await
            .expect("Failed to create audio query");

        let query = query.text().await.expect("Failed to get text");

        let params = [("speaker", &speaker)];
        let audio = client
            .post(format!("{}/synthesis", CONFIG.voicevox_host))
            .query(&params)
            .header(CONTENT_TYPE, "application/json")
            .body(query)
            .send()
            .await
            .expect("Failed to create audio query");

        let uuid = Uuid::new_v4().to_string();
        let path = Path::new(&CONFIG.tmp_path).join(&uuid);

        let mut output = File::create(&path).expect("Failed to create file");
        let audio = audio.bytes().await.expect("Failed to read resp");
        let mut response_cursor = Cursor::new(audio);
        io::copy(&mut response_cursor, &mut output).expect("Failed to write file");

        let source = match ffmpeg(&path).await {
            Ok(source) => source,
            Err(why) => {
                println!("Err starting source: {why:?}");
                return;
            }
        };

        let (audio, audio_handle) = create_player(source);

        audio_handle
            .add_event(
                Event::Track(TrackEvent::End),
                songbird_handler::ReadEndNotifier {
                    temporary_filename: path,
                },
            )
            .expect("Failed to create queue");

        handler.lock().await.enqueue(audio);
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match interaction {
            Interaction::ApplicationCommand(command) => match command.data.name.as_str() {
                "speaker" => commands::speaker::run(&ctx, command).await,
                "join" => commands::join::run(&ctx, command).await,
                "leave" => commands::leave::run(&ctx, command).await,
                "skip" => commands::skip::run(&ctx, command).await,
                _ => unreachable!("Unknown command: {}", command.data.name),
            },
            Interaction::MessageComponent(interaction) => {
                commands::speaker::update(&ctx, interaction).await;
            }
            _ => {}
        }
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    voicevox::load_speaker_info().await;

    let intents = GatewayIntents::GUILDS
        | GatewayIntents::GUILD_VOICE_STATES
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(&config::CONFIG.discord_token, intents)
        .event_handler(Handler)
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
