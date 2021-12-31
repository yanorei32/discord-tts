use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io;
use std::sync::{Arc, Mutex};

use once_cell::sync::OnceCell;
use reqwest::header::CONTENT_TYPE;
use serde::Deserialize;
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

#[macro_use]
extern crate lazy_static;

lazy_static! {
    static ref CURRENT_TEXT_CHANNEL: Mutex<HashMap<GuildId, ChannelId>> =
        Mutex::new(HashMap::new());
    static ref VOICE_OVERRIDE: Mutex<HashMap<UserId, String>> = Mutex::new(HashMap::new());
}

#[derive(Deserialize, Debug)]
struct Config {
    voicevox_host: String,
    discord_token: String,
}

static CONFIG: OnceCell<Config> = OnceCell::new();

#[group]
#[commands(join, leave, skip, set)]
struct General;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }

    async fn message(&self, ctx: Context, mut msg: Message) {
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

        let speaker = {
            let map = VOICE_OVERRIDE.lock().unwrap();
            match map.get(&msg.author.id) {
                Some(voice) => voice.clone(),
                None => "0".to_string(),
            }
        };

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

        let mut output = File::create(&uuid).expect("Failed to create file");
        let audio = audio.bytes().await.expect("Failed to read resp");
        let mut response_cursor = std::io::Cursor::new(audio);
        io::copy(&mut response_cursor, &mut output).expect("Failed to write file");

        let mut handler = handler.lock().await;

        let source = match ffmpeg(&uuid).await {
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
                    temporary_filename: uuid,
                },
            )
            .expect("Failed to create queue");

        handler.enqueue(audio);
    }
}

#[command]
#[only_in(guilds)]
async fn set(_ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let id = args.single::<i32>().expect("Failed");
    if !(0..=10).contains(&id) {
        return Ok(());
    }

    let mut map = VOICE_OVERRIDE.lock().unwrap();
    map.insert(msg.author.id, id.to_string());
    Ok(())
}

#[command]
#[only_in(guilds)]
async fn skip(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.unwrap();

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
    temporary_filename: String,
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
    let guild = msg.guild(&ctx.cache).await.unwrap();

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
    let guild = msg.guild(&ctx.cache).await.unwrap();

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

    let framework = StandardFramework::new()
        .configure(|c| c.prefix("~"))
        .group(&GENERAL_GROUP);

    let c = CONFIG.get().unwrap();
    let mut client = Client::builder(&c.discord_token)
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
