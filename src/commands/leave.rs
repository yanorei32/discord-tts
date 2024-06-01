use serenity::{builder::CreateCommand, client::Context, model::application::CommandInteraction};

use crate::commands::simple_resp_helper;

pub fn register(prefix: &str) -> CreateCommand {
    CreateCommand::new(format!("{prefix}leave"))
        .description("Leave from VC")
        .dm_permission(false)
}

pub async fn run(ctx: &Context, interaction: CommandInteraction) {
    let guild_id = interaction.guild_id.unwrap();

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird is not initialized.");

    let Ok(()) = manager.leave(guild_id).await else {
        simple_resp_helper(&interaction, ctx, "Not in a voice channel", true).await;
        return;
    };

    simple_resp_helper(&interaction, ctx, "Connection has been closed.", false).await;
}
