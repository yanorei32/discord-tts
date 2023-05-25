use serenity::{
    builder::CreateApplicationCommand,
    client::Context,
    model::{
        application::interaction::application_command::ApplicationCommandInteraction,
        prelude::Mentionable,
    },
};
use songbird::CoreEvent;

use crate::commands::simple_resp_helper;
use crate::db::INMEMORY_DB;
use crate::songbird_handler::DriverDisconnectNotifier;


pub fn register<'a>(prefix: &str, cmd: &'a mut CreateApplicationCommand) -> &'a mut CreateApplicationCommand {
    cmd.name(format!("{prefix}join"))
        .description("Join to your channel")
        .dm_permission(false)
}

pub async fn run(ctx: &Context, interaction: ApplicationCommandInteraction) {
    let guild = interaction.guild_id.unwrap().to_guild_cached(ctx).unwrap();

    let Some(Some(connect_to)) = guild.voice_states.get(&interaction.user.id).map(|v| v.channel_id) else {
        simple_resp_helper(&interaction, ctx, "You can use it only if you are in VC.", true).await;
        return;
    };

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird is not initialized.");

    if let Some(h) = manager.get(guild.id) {
        if h.lock().await.join(connect_to).await.is_err() {
            simple_resp_helper(&interaction, ctx, "Failed to rejoin to VC.", true).await;
            return;
        }
    } else {
        let (h, success) = manager.join(guild.id, connect_to).await;

        if success.is_err() {
            simple_resp_helper(&interaction, ctx, "Failed to join to VC.", true).await;
            return;
        };

        h.lock().await.add_global_event(
            CoreEvent::DriverDisconnect.into(),
            DriverDisconnectNotifier {
                songbird_manager: manager,
            },
        );
    }

    INMEMORY_DB.store_instance(guild.id, interaction.channel_id);

    simple_resp_helper(
        &interaction,
        ctx,
        &format!(
            "Linked! {} <-> {}",
            interaction.channel_id.mention(),
            connect_to.mention()
        ),
        false,
    )
    .await;
}
