use serenity::{
    builder::CreateApplicationCommand, client::Context,
    model::application::interaction::application_command::ApplicationCommandInteraction,
};

use crate::commands::simple_resp_helper;

pub fn register(cmd: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    cmd.name("skip")
        .description("Skip a current message")
        .dm_permission(false)
}

pub async fn run(ctx: &Context, interaction: ApplicationCommandInteraction) {
    let guild = interaction.guild_id.unwrap().to_guild_cached(ctx).unwrap();

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird is not initialized.");

    let Some(handler) = manager.get(guild.id) else {
        simple_resp_helper(&interaction, ctx, "Not in a voice channel.", true).await;
        return;
    };

    handler.lock().await.queue().skip().expect("Failed to skip");
    simple_resp_helper(&interaction, ctx, "Skipped!", true).await;
}
