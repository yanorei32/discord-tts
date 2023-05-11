use serenity::{
    client::Context,
    model::{
        application::interaction::{
            application_command::ApplicationCommandInteraction, InteractionResponseType,
        },
        prelude::interaction::MessageFlags,
    },
};

pub mod join;
pub mod leave;
pub mod skip;

async fn simple_resp_helper(
    interaction: &ApplicationCommandInteraction,
    ctx: &Context,
    text: &str,
    is_ephemeral: bool,
) {
    interaction
        .create_interaction_response(ctx, |resp| {
            resp.kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|mes| {
                    mes.content(text.to_string()).flags(if is_ephemeral {
                        MessageFlags::EPHEMERAL
                    } else {
                        MessageFlags::empty()
                    })
                })
        })
        .await
        .expect("Failed to write response");
}
