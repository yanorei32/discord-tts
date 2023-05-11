use crate::commands::simple_resp_helper;
use crate::WATCH_CHANNELS;

use serenity::{
    builder::CreateApplicationCommand, client::Context,
    model::application::interaction::application_command::ApplicationCommandInteraction,
};

pub fn register(cmd: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    cmd.name("leave")
        .description("Leave from VC")
        .dm_permission(false)
}

pub async fn run(ctx: &Context, interaction: ApplicationCommandInteraction) {
    let guild_id = interaction.guild_id.unwrap();

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird is not initialized.");

    if manager.get(guild_id).is_some() {
        manager
            .remove(guild_id)
            .await
            .expect("Failed to remove songbird instance");
        simple_resp_helper(&interaction, ctx, "Left a voice channel", false).await;
    } else {
        simple_resp_helper(&interaction, ctx, "Not in a voice channel", true).await;
    }

    WATCH_CHANNELS.lock().unwrap().remove(&guild_id);
}
