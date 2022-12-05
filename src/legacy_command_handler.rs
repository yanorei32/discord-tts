use serenity::{
    client::Context,
    framework::standard::{
        macros::{command, group},
        Args, CommandResult,
    },
    model::{channel::Message, prelude::Mentionable},
};
use songbird::CoreEvent;

use crate::serenity_utils::check_msg;
use crate::songbird_event_handler::DriverDisconnectNotifier;
use crate::CURRENT_TEXT_CHANNEL;

#[group]
#[commands(join, leave, skip, set)]
pub struct General;

#[command]
#[only_in(guilds)]
async fn join(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).unwrap();

    let Some(connect_to) = guild
        .voice_states
        .get(&msg.author.id)
        .and_then(|voice_state| voice_state.channel_id) else {
        check_msg(msg.reply(ctx, "Not in a voice channel").await);

        return Ok(());
    };

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    let (handler_lock, success) = manager.join(guild.id, connect_to).await;

    if let Err(_) = success {
        check_msg(
            msg.channel_id
                .say(&ctx.http, "Error joining the channel")
                .await,
        );

        return Ok(());
    };

    handler_lock.lock().await.add_global_event(
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

    CURRENT_TEXT_CHANNEL
        .lock()
        .unwrap()
        .insert(guild.id, msg.channel_id);

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
        let _ = handler_lock.lock().await.queue().skip();
    }

    Ok(())
}

#[command]
#[only_in(guilds)]
async fn set(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    check_msg(
        msg.reply(
            ctx,
            "This command is deprecated.\nPlease use a slash command /speaker change",
        )
        .await,
    );

    Ok(())
}

#[command]
#[only_in(guilds)]
async fn leave(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).unwrap();

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if manager.get(guild.id).is_some() {
        if let Err(e) = manager.remove(guild.id).await {
            check_msg(msg.reply(ctx, format!("Failed: {:?}", e)).await);
        }

        check_msg(msg.reply(ctx, "Left voice channel").await);
    } else {
        check_msg(msg.reply(ctx, "Not in a voice channel").await);
    }

    Ok(())
}
