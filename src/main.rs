#![warn(clippy::pedantic)]

mod commands;
mod config;
mod songbird_handler;
mod interactive_component;
mod message_filter;
mod model;
mod voicevox;

use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufReader, Write};
use std::path::Path;
use std::sync::Mutex;

use once_cell::sync::Lazy;
use reqwest::header::CONTENT_TYPE;
use serenity::{
    async_trait,
    builder::CreateInteractionResponseData,
    client::{Client, Context, EventHandler},
    model::{
        application::{
            command::{Command, CommandOptionType},
            interaction::{Interaction, InteractionResponseType},
        },
        channel::{AttachmentType::Bytes, Message},
        gateway::Ready,
        prelude::{ChannelId, GatewayIntents, GuildId, UserId},
    },
};
use songbird::{ffmpeg, tracks::create_player, Event, SerenityInit, TrackEvent};
use uuid::Uuid;

use crate::interactive_component::{CompileWithBuilder, SelectorResponse};
use crate::model::SpeakerSelector;
use crate::config::CONFIG;

static WATCH_CHANNELS: Lazy<Mutex<HashMap<GuildId, ChannelId>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

static STATE: Lazy<Mutex<model::State>> = Lazy::new(|| {
    Mutex::new(model::State {
        user_settings: HashMap::new(),
    })
});

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        Command::set_global_application_commands(&ctx.http, |commands| {
            commands
                .create_application_command(|command| commands::join::register(command))
                .create_application_command(|command| commands::leave::register(command))
                .create_application_command(|command| commands::skip::register(command))
                .create_application_command(|command| {
                    command
                        .name("speaker")
                        .description("Manage your speaker")
                        .create_option(|option| {
                            option
                                .kind(CommandOptionType::SubCommand)
                                .name("current")
                                .description("Show your current speaker")
                        })
                        .create_option(|option| {
                            option
                                .kind(CommandOptionType::SubCommand)
                                .name("change")
                                .description("Change your speaker")
                        })
                })
        })
        .await
        .unwrap();

        println!("{} is connected!", ready.user.name);
    }

    async fn message(&self, ctx: Context, msg: Message) {
        if msg.author.bot {
            return;
        }

        let Some(guild_id) = msg.guild_id else {
            return;
        };

        let Some(content) = message_filter::filter(msg.content.as_str()) else {
            return;
        };

        let manager = songbird::get(&ctx)
            .await
            .expect("Songbird Voice client placed in at init.")
            .clone();

        let Some(handler) = manager.get(guild_id) else {
            return;
        };

        if WATCH_CHANNELS
            .lock()
            .unwrap()
            .get(&guild_id)
            .map_or(true, |id| id != &msg.channel_id)
        {
            return;
        }

        let speaker = get_speaker_id(msg.author.id).to_string();

        let params = [("text", &content), ("speaker", &speaker)];
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
        let mut response_cursor = std::io::Cursor::new(audio);
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
                "speaker" => match command.data.options.first() {
                    None => unreachable!(),
                    _ => match command.data.options.first().unwrap().name.as_str() {
                        "current" => {
                            command
                                .create_interaction_response(&ctx.http, |response| {
                                    response
                                        .kind(InteractionResponseType::ChannelMessageWithSource)
                                        .interaction_response_data(|message| {
                                            build_current_speaker_response(
                                                message,
                                                command.user.id,
                                            );
                                            message
                                        })
                                })
                                .await
                                .expect("Failed to create response");
                        }
                        "change" => {
                            command
                                .create_interaction_response(&ctx.http, |response| {
                                    response
                                        .kind(InteractionResponseType::ChannelMessageWithSource)
                                        .interaction_response_data(|message| {
                                            build_speaker_selector_response(
                                                message,
                                                SpeakerSelector::None,
                                            );
                                            message
                                        })
                                })
                                .await
                                .expect("Failed to create response");
                        }
                        _ => unreachable!(),
                    },
                },
                "join" => commands::join::run(&ctx, command).await,
                "leave" => commands::leave::run(&ctx, command).await,
                "skip" => commands::skip::run(&ctx, command).await,
                _ => unreachable!("Unknown command: {}", command.data.name),
            },
            Interaction::MessageComponent(interaction) => {
                if interaction.data.custom_id.contains("select_style") {
                    interaction
                        .create_interaction_response(&ctx.http, |response| {
                            let style_id: String =
                                interaction.data.custom_id.chars().skip(13).collect();
                            let style_id: u8 = style_id.parse().unwrap();

                            {
                                let mut state = STATE.lock().unwrap();
                                let mut settings =
                                    match state.user_settings.get(&interaction.user.id) {
                                        Some(settings) => *settings,
                                        None => model::UserSettings { speaker: None },
                                    };

                                settings.speaker = Some(style_id);
                                state.user_settings.insert(interaction.user.id, settings);
                            }
                            save_state();

                            response
                                .kind(InteractionResponseType::UpdateMessage)
                                .interaction_response_data(|message| {
                                    build_current_speaker_response(message, interaction.user.id);
                                    message.components(|components| components)
                                })
                        })
                        .await
                        .expect("Failed to create response");
                } else if interaction.data.custom_id.contains("speaker") {
                    interaction
                        .create_interaction_response(&ctx.http, |response| {
                            let values = &interaction.data.values;
                            let index: usize = values.get(0).unwrap().parse().unwrap();

                            response
                                .kind(InteractionResponseType::UpdateMessage)
                                .interaction_response_data(|message| {
                                    build_speaker_selector_response(
                                        message,
                                        SpeakerSelector::SpeakerOnly { speaker: index },
                                    );
                                    message
                                })
                        })
                        .await
                        .expect("Failed to create response");
                } else if interaction.data.custom_id.contains("style") {
                    interaction
                        .create_interaction_response(&ctx.http, |response| {
                            let values = &interaction.data.values;
                            let indices: Vec<&str> = values.get(0).unwrap().split('_').collect();
                            let speaker_index: usize = indices.first().unwrap().parse().unwrap();
                            let style_index: usize = indices.get(1).unwrap().parse().unwrap();

                            response
                                .kind(InteractionResponseType::UpdateMessage)
                                .interaction_response_data(|message| {
                                    build_speaker_selector_response(
                                        message,
                                        SpeakerSelector::SpeakerAndStyle {
                                            speaker: speaker_index,
                                            style: style_index,
                                        },
                                    );
                                    message
                                })
                        })
                        .await
                        .expect("Failed to create response");
                }
            }
            _ => {}
        }
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    load_state();
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

fn save_state() {
    let mut f = File::create(&CONFIG.state_path).expect("Unable to open file.");

    let s = STATE.lock().unwrap();
    f.write_all(
        serde_json::to_string(&s.user_settings)
            .expect("Failed to serialize")
            .as_bytes(),
    )
    .expect("Unable to write data");
}

fn load_state() {
    match File::open(&CONFIG.state_path) {
        Ok(f) => {
            let reader = BufReader::new(f);
            let mut s = STATE.lock().unwrap();
            s.user_settings = serde_json::from_reader(reader).expect("JSON was not well-formatted");
        }
        Err(_) => {
            println!("Failed to read state.json");
        }
    }
}

fn get_speaker_id(user_id: UserId) -> u8 {
    STATE
        .lock()
        .unwrap()
        .user_settings
        .get(&user_id)
        .and_then(|s| s.speaker)
        .unwrap_or(0)
}

fn build_current_speaker_response(message: &mut CreateInteractionResponseData, user_id: UserId) {
    let speaker_id = get_speaker_id(user_id);
    let speakers = voicevox::get_speakers();

    for speaker in &speakers {
        if let Some(style) = speaker
            .styles
            .iter()
            .find(|style| style.id == u32::from(speaker_id))
        {
            message
                .add_file(Bytes {
                    data: style.icon.clone(),
                    filename: "icon.png".to_string(),
                })
                .embed(|embed| {
                    embed
                        .author(|author| author.name("Speaker currently in use"))
                        .thumbnail("attachment://icon.png")
                        .fields([
                            ("Speaker name", &speaker.name, false),
                            ("Style", &style.name, true),
                            ("id", &style.id.to_string(), true),
                        ])
                })
                .ephemeral(true);
            break;
        }
    }
}

fn build_speaker_selector_response(
    message: &mut CreateInteractionResponseData,
    selector: SpeakerSelector,
) {
    let speakers = voicevox::get_speakers();

    let message = match selector {
        SpeakerSelector::SpeakerAndStyle {
            speaker: speaker_index,
            style,
        } => {
            let speaker = speakers.get(speaker_index).unwrap();
            let style = speaker.styles.get(style).unwrap();

            message.add_file(Bytes {
                data: style.icon.clone(),
                filename: "thumbnail.png".to_string(),
            });

            style
                .samples
                .iter()
                .enumerate()
                .fold(message, |m, (i, sample)| {
                    m.add_file(Bytes {
                        data: sample.clone(),
                        filename: format!("sample{i}.wav"),
                    })
                })
        }
        SpeakerSelector::SpeakerOnly { speaker: index } => {
            let speaker = speakers.get(index).unwrap();

            message.add_file(Bytes {
                data: speaker.portrait.clone(),
                filename: "thumbnail.png".to_string(),
            })
        }
        SpeakerSelector::None => message,
    };

    if let Some(speaker_index) = selector.speaker() {
        let speaker = speakers.get(speaker_index).unwrap();

        message.embed(|embed| {
            embed
                .author(|author| author.name("Select speaker you want to use"))
                .thumbnail("attachment://thumbnail.png")
                .field("Name", &speaker.name, true);

            let style = selector.style().map(|a| speaker.styles.get(a).unwrap());
            embed.fields([
                (
                    "Style",
                    style.map_or_else(|| "-".to_string(), |s| s.name.clone()),
                    true,
                ),
                (
                    "ID",
                    style.map_or_else(|| "-".to_string(), |s| s.id.to_string()),
                    true,
                ),
                ("Policy", speaker.policy.clone(), false),
            ])
        });
    }

    SelectorResponse::default().build((speakers, selector), message);
}
