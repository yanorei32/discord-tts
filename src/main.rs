extern crate core;

mod voicevox;

use std::collections::HashMap;
use std::fmt::Debug;
use std::fs::{self, File};
use std::io::{self, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::voicevox::Speaker;
use once_cell::sync::{Lazy, OnceCell};
use reqwest::header::CONTENT_TYPE;
use serde::{Deserialize, Serialize};
use serenity::builder::CreateInteractionResponse;
use serenity::model::application::command::CommandOptionType;
use serenity::model::application::interaction::Interaction;
use serenity::model::application::interaction::InteractionResponseType;
use serenity::model::channel::AttachmentType::Bytes;
use serenity::model::prelude::component::ButtonStyle;
use serenity::prelude::GatewayIntents;
use serenity::{
    async_trait,
    client::{Client, Context, EventHandler},
    framework::{
        standard::{
            macros::{command, group},
            Args, CommandResult,
        },
        StandardFramework,
    },
    model::{
        channel::Message,
        gateway::Ready,
        prelude::{ChannelId, GuildId, Mentionable, UserId},
    },
    Result as SerenityResult,
};
use songbird::{
    ffmpeg, tracks::create_player, CoreEvent, Event, EventContext,
    EventHandler as VoiceEventHandler, SerenityInit, Songbird, TrackEvent,
};
use uuid::Uuid;

static CURRENT_TEXT_CHANNEL: Lazy<Mutex<HashMap<GuildId, ChannelId>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
static STATE: Lazy<Mutex<State>> = Lazy::new(|| {
    Mutex::new(State {
        user_settings: HashMap::new(),
    })
});

#[derive(Deserialize, Debug)]
struct Config {
    voicevox_host: String,
    discord_token: String,
    state_path: String,
    tmp_path: String,
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
struct UserSettings {
    speaker: Option<u8>,
}

#[derive(Serialize, Deserialize, Debug)]
struct State {
    user_settings: HashMap<UserId, UserSettings>,
}

static CONFIG: OnceCell<Config> = OnceCell::new();

#[group]
#[commands(join, leave, skip, set)]
struct General;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        let _ = GuildId(563378729422946305)
            .create_application_command(&ctx.http, |c| {
                c.name("character")
                    .description("Manage your speaking character")
                    .create_option(|option| {
                        option
                            .kind(CommandOptionType::SubCommand)
                            .name("current")
                            .description("Your current speaking character")
                    })
                    .create_option(|option| {
                        option
                            .kind(CommandOptionType::SubCommand)
                            .name("change")
                            .description("Change your speaking character")
                    })
            })
            .await
            .expect("");

        println!("{} is connected!", ready.user.name);
    }

    async fn message(&self, ctx: Context, msg: Message) {
        if msg.author.bot {
            return;
        }

        if msg.content == "ping" {
            check_msg(msg.channel_id.say(&ctx.http, "[discord-tts] pong").await);
            return;
        }

        match msg.content.get(..1) {
            Some("~") => return,
            Some(";") => match msg.content.chars().nth(1) {
                Some(';') => {}
                _ => return,
            },
            _ => {}
        };

        let guild_id = match msg.guild_id {
            Some(guild_id) => guild_id,
            None => return,
        };

        let manager = songbird::get(&ctx)
            .await
            .expect("Songbird Voice client placed in at initialisation.")
            .clone();

        let handler = match manager.get(guild_id) {
            Some(handler) => handler,
            None => return,
        };

        {
            let m = CURRENT_TEXT_CHANNEL.lock().unwrap();
            match m.get(&guild_id) {
                Some(channel_id) => {
                    if channel_id != &msg.channel_id {
                        return;
                    }
                }
                None => return,
            }
        }

        let speaker = get_speaker_id(&msg.author.id).to_string();

        let c = CONFIG.get().unwrap();

        let params = [("text", msg.content.as_str()), ("speaker", &speaker)];
        let client = reqwest::Client::new();
        let query = client
            .post(format!("{}/audio_query", c.voicevox_host))
            .query(&params)
            .send()
            .await
            .expect("Failed to create audio query");

        let query = query.text().await.expect("Failed to get text");

        let params = [("speaker", &speaker)];
        let audio = client
            .post(format!("{}/synthesis", c.voicevox_host))
            .query(&params)
            .header(CONTENT_TYPE, "application/json")
            .body(query)
            .send()
            .await
            .expect("Failed to create audio query");

        let uuid = Uuid::new_v4().to_string();
        let path = Path::new(&c.tmp_path).join(&uuid);

        let mut output = File::create(&path).expect("Failed to create file");
        let audio = audio.bytes().await.expect("Failed to read resp");
        let mut response_cursor = std::io::Cursor::new(audio);
        io::copy(&mut response_cursor, &mut output).expect("Failed to write file");

        let mut handler = handler.lock().await;

        let source = match ffmpeg(&path).await {
            Ok(source) => source,
            Err(why) => {
                println!("Err starting source: {:?}", why);
                check_msg(msg.reply(ctx, "Error sourcing ffmpeg").await);
                return;
            }
        };

        let (audio, audio_handle) = create_player(source);

        audio_handle
            .add_event(
                Event::Track(TrackEvent::End),
                ReadEndNotifier {
                    temporary_filename: path,
                },
            )
            .expect("Failed to create queue");

        handler.enqueue(audio);
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            match command.data.name.as_str() {
                "character" => {
                    match command.data.options.first() {
                        None => unreachable!(),
                        _ => match command.data.options.first().unwrap().name.as_str() {
                            "current" => {
                                let speaker_id = get_speaker_id(&command.user.id);
                                let speakers = voicevox::get_speakers();

                                'speaker: for speaker in &speakers {
                                    for style in &speaker.styles {
                                        if style.id == u32::from(speaker_id) {
                                            let _ = command.create_interaction_response(&ctx.http, |response| {
                                                    response.kind(InteractionResponseType::ChannelMessageWithSource)
                                                        .interaction_response_data(|message| {
                                                            message.add_file(Bytes {
                                                                data: style.icon.clone(),
                                                                filename: "icon.png".to_string(),
                                                            })
                                                                .embed(|embed| {
                                                                    embed.author(|author| {
                                                                        author.name("Character currently in use")
                                                                    })
                                                                        .thumbnail("attachment://icon.png")
                                                                        .field("Character name", &speaker.name, false)
                                                                        .field("Style", &style.name, true)
                                                                        .field("id", &style.id, true)
                                                                })
                                                                .ephemeral(true)
                                                        })
                                                }).await;
                                            break 'speaker;
                                        }
                                    }
                                }
                            }
                            "change" => {
                                let _ = command
                                    .create_interaction_response(&ctx.http, |response| {
                                        get_speaker_response(response, 0);
                                        response
                                    })
                                    .await;
                            }
                            _ => unreachable!(),
                        },
                    }

                    let _ = command
                        .create_interaction_response(&ctx.http, |response| {
                            response
                                .kind(InteractionResponseType::ChannelMessageWithSource)
                                .interaction_response_data(|message| {
                                    message
                                        .content(format!("{}", command.data.options.len()))
                                        .ephemeral(true)
                                })
                        })
                        .await;
                }
                _ => unreachable!("Unknown command: {}", command.data.name),
            }
        }
    }
}

#[command]
#[only_in(guilds)]
async fn set(_ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let id = args.single::<u8>().expect("Failed");
    if !(0..=38).contains(&id) {
        return Ok(());
    }

    {
        let mut s = STATE.lock().unwrap();

        let mut settings: UserSettings = match s.user_settings.get(&msg.author.id) {
            Some(settings) => settings.clone(),
            None => UserSettings { speaker: None },
        };

        settings.speaker = Some(id);
        s.user_settings.insert(msg.author.id, settings);
    }

    save_state();

    Ok(())
}

#[command]
#[only_in(guilds)]
async fn skip(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let guild = msg.guild(&ctx.cache).unwrap();

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if let Some(handler_lock) = manager.get(guild.id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();
        let _ = queue.skip();
    }

    Ok(())
}

struct ReadEndNotifier {
    temporary_filename: PathBuf,
}

#[async_trait]
impl VoiceEventHandler for ReadEndNotifier {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        if let EventContext::Track(_) = ctx {
            fs::remove_file(&self.temporary_filename).expect("Failed to remove temporary file")
        }
        None
    }
}

struct DriverDisconnectNotifier {
    songbird_manager: Arc<Songbird>,
}

#[async_trait]
impl VoiceEventHandler for DriverDisconnectNotifier {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        if let EventContext::DriverDisconnect(ctx) = ctx {
            let guild_id = ctx.guild_id;
            let manager = &self.songbird_manager;
            let has_handler = manager.get(guild_id).is_some();

            println!("Force disconnected");

            if has_handler {
                manager
                    .remove(guild_id)
                    .await
                    .expect("Failed to remove from manager");
            }
        }
        None
    }
}

#[command]
#[only_in(guilds)]
async fn leave(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).unwrap();

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    let has_handler = manager.get(guild.id).is_some();

    if has_handler {
        if let Err(e) = manager.remove(guild.id).await {
            check_msg(
                msg.channel_id
                    .say(&ctx.http, format!("Failed: {:?}", e))
                    .await,
            );
        }

        check_msg(msg.reply(ctx, "Left voice channel").await);
    } else {
        check_msg(msg.reply(ctx, "Not in a voice channel").await);
    }

    Ok(())
}

#[command]
#[only_in(guilds)]
async fn join(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).unwrap();

    let channel_id = guild
        .voice_states
        .get(&msg.author.id)
        .and_then(|voice_state| voice_state.channel_id);

    let connect_to = match channel_id {
        Some(channel) => channel,
        None => {
            check_msg(msg.reply(ctx, "Not in a voice channel").await);
            return Ok(());
        }
    };

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    let (handler_lock, success) = manager.join(guild.id, connect_to).await;

    if let Ok(_channel) = success {
        let mut handler = handler_lock.lock().await;
        handler.add_global_event(
            CoreEvent::DriverDisconnect.into(),
            DriverDisconnectNotifier {
                songbird_manager: manager.clone(),
            },
        );

        check_msg(
            msg.channel_id
                .say(
                    &ctx.http,
                    &format!(
                        r#"
**Joined** {}

VOICEVOX
```
VOICEVOX:四国めたん|VOICEVOX:ずんだもん: https://zunko.jp/con_ongen_kiyaku.html
VOICEVOX:春日部つむぎ: https://tsukushinyoki10.wixsite.com/ktsumugiofficial/%E5%88%A9%E7%94%A8%E8%A6%8F%E7%B4%84
VOICEVOX:雨晴はう: https://amehau.com/?page_id=225
VOICEVOX:波音リツ: http://canon-voice.com/kiyaku.html
```
                        "#,
                        connect_to.mention()
                    ),
                )
                .await,
        );

        let mut map = CURRENT_TEXT_CHANNEL.lock().unwrap();
        map.insert(guild.id, msg.channel_id);
    } else {
        check_msg(
            msg.channel_id
                .say(&ctx.http, "Error joining the channel")
                .await,
        );
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    CONFIG
        .set(envy::from_env::<Config>().expect("Failed to get environment"))
        .unwrap();

    load_state();
    voicevox::load_speaker_info().await;

    let framework = StandardFramework::new()
        .configure(|c| c.prefix("~"))
        .group(&GENERAL_GROUP);

    let c = CONFIG.get().unwrap();
    let intents = GatewayIntents::GUILDS
        | GatewayIntents::GUILD_VOICE_STATES
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(&c.discord_token, intents)
        .event_handler(Handler)
        .framework(framework)
        .register_songbird()
        .await
        .expect("Failed to create client");

    tokio::spawn(async move {
        let _ = client
            .start()
            .await
            .map_err(|why| println!("Client ended: {:?}", why));
    });

    tokio::signal::ctrl_c()
        .await
        .expect("Failed to wait Ctrl+C");

    println!("Received Ctrl+C, shutting down.");
}

fn check_msg(result: SerenityResult<Message>) {
    if let Err(why) = result {
        println!("Error sending message: {:?}", why);
    }
}

fn save_state() {
    let c = CONFIG.get().unwrap();
    let mut f = File::create(&c.state_path).expect("Unable to open file.");

    let s = STATE.lock().unwrap();
    f.write_all(
        serde_json::to_string(&s.user_settings)
            .expect("Failed to serialize")
            .as_bytes(),
    )
    .expect("Unable to write data");
}

fn load_state() {
    let c = CONFIG.get().unwrap();
    match File::open(&c.state_path) {
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

fn get_speaker_id(user_id: &UserId) -> u8 {
    let state = STATE.lock().unwrap();
    match state.user_settings.get(user_id) {
        Some(settings) => match settings.speaker {
            Some(speaker) => speaker,
            _ => 0,
        },
        None => 0,
    }
}

fn get_speaker_response(response: &mut CreateInteractionResponse, position: u32) {
    let speakers = voicevox::get_speakers();

    let speaker = (speakers.get(position as usize) as Option<&Speaker>).unwrap();

    response
        .kind(InteractionResponseType::ChannelMessageWithSource)
        .interaction_response_data(|message| {
            message
                .add_file(Bytes {
                    data: speaker.portrait.clone(),
                    filename: "portrait.png".to_string(),
                })
                .embed(|embed| {
                    embed
                        .author(|author| author.name("Select character you want to use"))
                        .thumbnail("attachment://portrait.png")
                        .field("Character name", speaker.name.clone(), false)
                        .field("Policy", speaker.policy.clone(), false)
                })
                .components(|component| {
                    component.create_action_row(|action| {
                        action
                            .create_button(|button| {
                                if position == 0 {
                                    button
                                        .style(ButtonStyle::Secondary)
                                        .custom_id("previous_character_disabled")
                                        .label("Previous character")
                                } else {
                                    button
                                        .style(ButtonStyle::Primary)
                                        .custom_id(format!("previous_character_{}", position - 1))
                                        .label("Previous character")
                                }
                            })
                            .create_button(|button| {
                                button
                                    .style(ButtonStyle::Success)
                                    .custom_id(format!("select_character_{}", position))
                                    .label("Select this character")
                            })
                            .create_button(|button| {
                                if position == speakers.len() as u32 - 1 {
                                    button
                                        .style(ButtonStyle::Secondary)
                                        .custom_id("next_character_disabled")
                                        .label("Next character")
                                } else {
                                    button
                                        .style(ButtonStyle::Primary)
                                        .custom_id(format!("next_character_{}", position + 1))
                                        .label("Next character")
                                }
                            })
                    })
                })
                .ephemeral(true)
        });
}
