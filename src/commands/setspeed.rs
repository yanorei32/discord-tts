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
    CreateCommand::new(format!("{prefix}setspeed"))
        .description("Set your speaking speed")
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
    let speed = interaction
        .data
        .options
        .first()
        .and_then(|opt| opt.value.as_f64())
        .unwrap() as f32;

    PERSISTENT_DB.store_speed(interaction.user.id, speed);

    let response = CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
            .content(format!("Speaking speed set to {:.2}", speed))
            .ephemeral(true),
    );

    interaction
        .create_response(&ctx.http, response)
        .await
        .unwrap();
}
