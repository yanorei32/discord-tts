use serenity::{
    all::InteractionResponseFlags,
    builder::{CreateInteractionResponse, CreateInteractionResponseMessage},
    client::Context,
    model::application::CommandInteraction,
};

pub mod join;
pub mod leave;
pub mod skip;
pub mod speaker;
pub mod setspeed;
pub mod setdefaultspeed;

async fn simple_resp_helper(
    interaction: &CommandInteraction,
    ctx: &Context,
    text: &str,
    is_ephemeral: bool,
) {
    interaction
        .create_response(
            ctx,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content(text.to_string())
                    .flags(if is_ephemeral {
                        InteractionResponseFlags::EPHEMERAL
                    } else {
                        InteractionResponseFlags::empty()
                    }),
            ),
        )
        .await
        .expect("Failed to write response");
}
