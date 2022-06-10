use std::fs::File;
use std::ops::DerefMut;
use std::path::Path;
use reqwest::header::CONTENT_TYPE;
use serenity::async_trait;
use serenity::client::{Context, EventHandler};
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use songbird::{Call, create_player, ffmpeg, TrackEvent};
use songbird::Event;
use uuid::Uuid;
use crate::{listener::songbird::ReadEndNotifier, CONFIG, CURRENT_TEXT_CHANNEL, ON_MEMORY_SETTING, Config};
use crate::log_serenity_error::LogSerenityError;

pub struct Handler;

/// invoke POST request.
/// returns audio source that can be passed to ffmpeg function.
async fn get_audio(text: impl AsRef<str> + Send + Sync, speaker: u8, config: &Config) -> reqwest::Result<bytes::Bytes> {
    let client = reqwest::Client::new();
    let query = client
        .post(format!("{}/audio_query", config.voicevox_host))
        .query(&[("text", text.as_ref())])
        .query(&[("speaker", &speaker)])
        .send()
        .await
        .expect("Failed to create audio query");

    // NOTE: we do not have to deserialize the response
    // we pass it directly to `POST /synthesis`
    let query = query.text().await.expect("Failed to get text");

    let params = [("speaker", &speaker)];
    let audio = client
        .post(format!("{}/synthesis", config.voicevox_host))
        .query(&params)
        .header(CONTENT_TYPE, "application/json")
        .body(query)
        .send()
        .await
        .expect("Failed to create audio query");

    audio.bytes().await
}

async fn queue_audio(mut call: impl DerefMut<Target=Call>, path: impl AsRef<Path>, msg: Message, ctx: Context) {
    let handler = &mut *call;

    let source = match ffmpeg(path.as_ref()).await {
        Ok(source) => source,
        Err(why) => {
            println!("Err starting source: {why:?}");
            msg.reply(ctx, "Error sourcing ffmpeg").await.log_error();
            return;
        }
    };

    let (audio, audio_handle) = create_player(source);

    audio_handle
        .add_event(
            Event::Track(TrackEvent::End),
            ReadEndNotifier {
                temporary_filename: path.as_ref().to_path_buf(),
            },
        )
        .expect("Failed to create queue");

    handler.enqueue(audio);
}

/// write all. Returns the temporary file's path.
fn write_bytes_to_temporary_file(audio: bytes::Bytes, config: &Config) -> impl AsRef<Path> {
    let path = Path::new(&config.tmp_path).join(Uuid::new_v4().to_string());

    let mut output = File::create(&path).expect("Failed to create file");
    let mut response_cursor = std::io::Cursor::new(audio);
    std::io::copy(&mut response_cursor, &mut output).expect("Failed to write file");

    path
}

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
            msg.channel_id.say(&ctx.http, "[discord-tts] pong").await.log_error();
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

        let manager = &songbird::get(&ctx)
            .await
            .expect("Songbird Voice client placed in at initialisation.");

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

        let speaker = ON_MEMORY_SETTING.get()
            .unwrap()
            .lock()
            .unwrap()
            .state
            .user_settings
            .get(&msg.author.id)
            .and_then(|setting| setting.speaker)
            .unwrap_or(0);

        let content = msg.content.as_str();

        let audio = get_audio(content, speaker).await.expect("Failed to read resp");
        let config = CONFIG.get().unwrap();
        let audio = get_audio(content, speaker, config).await.expect("Failed to read resp");
        let path = write_bytes_to_temporary_file(audio, config);
        queue_audio(handler.lock().await, path, msg, ctx).await;
    }
}
