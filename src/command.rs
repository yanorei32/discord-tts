use serenity::client::Context;
use serenity::framework::standard::{Args, CommandResult, macros::{command, group}};
use serenity::model::channel::Message;
use serenity::prelude::Mentionable;
use songbird::CoreEvent;
use crate::{CONFIG, CURRENT_TEXT_CHANNEL, listener::songbird::DriverDisconnectNotifier, ON_MEMORY_SETTING, UserSettings};
use crate::log_serenity_error::LogSerenityError;

#[group]
#[commands(join, leave, skip, set)]
struct General;

#[command]
#[only_in(guilds)]
async fn set(_ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let id = args.single::<u8>().expect("Failed");
    if !(0..=10).contains(&id) {
        return Ok(());
    }

    {
        let s = &mut ON_MEMORY_SETTING.get().unwrap().lock().unwrap().state;

        let mut settings = s.user_settings.get(&msg.author.id).copied().unwrap_or(UserSettings { speaker: None });
        settings.speaker = Some(id);
        s.user_settings.insert(msg.author.id, settings);
    }

    ON_MEMORY_SETTING.get().unwrap().lock().unwrap().save(CONFIG.get().unwrap());

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
            msg.channel_id
                .say(&ctx.http, format!("Failed: {:?}", e))
                .await
                .log_error();
        }

        msg.reply(ctx, "Left voice channel").await.log_error();
    } else {
        msg.reply(ctx, "Not in a voice channel").await.log_error();
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
            msg.reply(ctx, "Not in a voice channel").await.log_error();
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
            .await
            .log_error();

        let mut map = CURRENT_TEXT_CHANNEL.lock().unwrap();
        map.insert(guild.id, msg.channel_id);
    } else {
        msg.channel_id
            .say(&ctx.http, "Error joining the channel")
            .await
            .log_error();
    }

    Ok(())
}
