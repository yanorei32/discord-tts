use std::fs::File;
use std::path::Path;
use reqwest::header::CONTENT_TYPE;
use serenity::async_trait;
use serenity::client::{Context, EventHandler};
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use songbird::{create_player, ffmpeg, TrackEvent};
use songbird::Event;
use uuid::Uuid;
use crate::{check_msg, listener::songbird::ReadEndNotifier, CONFIG, CURRENT_TEXT_CHANNEL, ON_MEMORY_SETTING};

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
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

        let speaker = {
            let s = ON_MEMORY_SETTING.get().unwrap().lock().unwrap();
            match s.state.user_settings.get(&msg.author.id) {
                Some(setting) => match setting.speaker {
                    Some(speaker) => speaker,
                    _ => 0,
                },
                None => 0,
            }
                .to_string()
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
        let path = Path::new(&c.tmp_path).join(&uuid);

        let mut output = File::create(&path).expect("Failed to create file");
        let audio = audio.bytes().await.expect("Failed to read resp");
        let mut response_cursor = std::io::Cursor::new(audio);
        std::io::copy(&mut response_cursor, &mut output).expect("Failed to write file");

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
}
