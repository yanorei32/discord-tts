use serenity::{builder::CreateCommand, client::Context, model::application::CommandInteraction};

use crate::commands::simple_resp_helper;

pub fn register(prefix: &str) -> CreateCommand {
    CreateCommand::new(format!("{prefix}skip"))
        .description("Skip a current message")
        .dm_permission(false)
}

pub async fn run(ctx: &Context, interaction: CommandInteraction) {
    let guild_id = interaction.guild_id.unwrap();

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird is not initialized.");

    let Some(handler) = manager.get(guild_id) else {
        simple_resp_helper(&interaction, ctx, "Not in a voice channel.", true).await;
        return;
    };

    handler.lock().await.queue().skip().expect("Failed to skip");
    simple_resp_helper(&interaction, ctx, "Skipped!", true).await;
}
