use serenity::{
    builder::CreateCommand,
    client::Context,
    model::{application::CommandInteraction, id::ChannelId, prelude::Mentionable, Permissions},
};
use songbird::CoreEvent;

use crate::commands::simple_resp_helper;
use crate::db::INMEMORY_DB;
use crate::songbird_handler::DriverDisconnectNotifier;

pub fn register(prefix: &str) -> CreateCommand {
    CreateCommand::new(format!("{prefix}join"))
        .description("Join to your channel")
        .dm_permission(false)
}

#[allow(clippy::enum_variant_names)]
enum JoinError {
    YouAreNotInVoiceChannel,
    FailedToJoinVoiceChannel,
    CannotAccessToTextChannel(ChannelId),
    CannotAccessToVoiceChannel(ChannelId),
}

impl JoinError {
    fn to_message(&self) -> String {
        match self {
            Self::YouAreNotInVoiceChannel => "You are not in voice channel".to_string(),
            Self::FailedToJoinVoiceChannel => "Failed to join to voice channel".to_string(),
            Self::CannotAccessToTextChannel(id) | Self::CannotAccessToVoiceChannel(id) => {
                format!("Cannot access to {}", id.mention())
            }
        }
    }
}

async fn run_(
    ctx: &Context,
    interaction: &CommandInteraction,
) -> Result<(ChannelId, ChannelId), JoinError> {
    if !interaction
        .app_permissions
        .unwrap()
        .contains(Permissions::VIEW_CHANNEL)
    {
        return Err(JoinError::CannotAccessToTextChannel(interaction.channel_id));
    }

    let guild = ctx
        .cache
        .guild(interaction.guild_id.unwrap())
        .unwrap()
        .clone();

    let vc = guild
        .voice_states
        .get(&interaction.user.id)
        .map(|v| guild.channels.get(&v.channel_id.unwrap()).unwrap().clone())
        .ok_or(JoinError::YouAreNotInVoiceChannel)?;

    if !vc
        .permissions_for_user(&ctx.cache, ctx.cache.current_user().id)
        .unwrap()
        .contains(Permissions::VIEW_CHANNEL | Permissions::CONNECT | Permissions::SPEAK)
    {
        return Err(JoinError::CannotAccessToVoiceChannel(vc.id));
    }

    let manager = songbird::get(ctx).await.unwrap();

    if let Some(h) = manager.get(guild.id) {
        h.lock()
            .await
            .join(vc.id)
            .await
            .map_err(|_| JoinError::FailedToJoinVoiceChannel)?;
    } else {
        let h = manager
            .join(guild.id, vc.id)
            .await
            .map_err(|_| JoinError::FailedToJoinVoiceChannel)?;

        h.lock().await.add_global_event(
            CoreEvent::DriverDisconnect.into(),
            DriverDisconnectNotifier {
                songbird_manager: manager,
            },
        );
    }

    INMEMORY_DB.store_instance(guild.id, interaction.channel_id);

    Ok((interaction.channel_id, vc.id))
}

pub async fn run(ctx: &Context, interaction: CommandInteraction) {
    match run_(ctx, &interaction).await {
        Ok((text, voice)) => {
            simple_resp_helper(
                &interaction,
                ctx,
                &format!("Linked! {} <-> {}", text.mention(), voice.mention()),
                false,
            )
            .await;
        }
        Err(e) => simple_resp_helper(&interaction, ctx, &e.to_message(), true).await,
    }
}
