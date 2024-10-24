use serenity::{
    builder::{
        CreateCommand, CreateCommandOption, CreateInteractionResponse,
        CreateInteractionResponseMessage,
    },
    client::Context,
    model::application::{CommandInteraction, CommandOptionType},
};

use crate::db::PERSISTENT_DB;

pub fn register(prefix: &str) -> CreateCommand {
    CreateCommand::new(format!("{prefix}setdefaultspeed"))
        .description("Set the default speaking speed for your guild")
        .dm_permission(false)
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::Number,
                "speed",
                "Speaking speed (1.0 to 2.0)",
            )
            .required(true)
            .min_number_value(1.0)
            .max_number_value(2.0),
        )
}

pub async fn run(ctx: &Context, interaction: CommandInteraction) {
    // Check if the user has the MANAGE_GUILD permission
    if !interaction
        .member
        .as_ref()
        .unwrap()
        .permissions
        .unwrap()
        .manage_guild()
    {
        let response = CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .content("You need the Manage Server permission to use this command.")
                .ephemeral(true),
        );
        interaction
            .create_response(&ctx.http, response)
            .await
            .unwrap();
        return;
    }
    let speed = interaction
        .data
        .options
        .first()
        .and_then(|opt| opt.value.as_f64())
        .unwrap() as f32;

    PERSISTENT_DB.store_speed_default(interaction.guild_id.unwrap(), speed);

    let response = CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
            .content(format!("Default speaking speed set to {:.2}", speed))
            .ephemeral(true),
    );

    interaction
        .create_response(&ctx.http, response)
        .await
        .unwrap();
}
