use crate::model::SpeakerSelector;

use serenity::{
    builder::CreateApplicationCommand,
    client::Context,
    model::application::{
        command::CommandOptionType,
        interaction::{
            application_command::ApplicationCommandInteraction, InteractionResponseType,
        },
    },
};

pub fn register(cmd: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    cmd.name("speaker")
        .description("Manage your speaker")
        .create_option(|opt| {
            opt.kind(CommandOptionType::SubCommand)
                .name("current")
                .description("Show your current speaker")
        })
        .create_option(|opt| {
            opt.kind(CommandOptionType::SubCommand)
                .name("change")
                .description("Change your speaker")
        })
}

pub async fn run(ctx: &Context, interaction: ApplicationCommandInteraction) {
    match interaction.data.options.first().unwrap().name.as_str() {
        "current" => interaction
            .create_interaction_response(&ctx.http, |response| {
                response
                    .kind(InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|mes| {
                        crate::build_current_speaker_response(mes, interaction.user.id);
                        mes
                    })
            })
            .await
            .expect("Failed to create response"),
        "change" => interaction
            .create_interaction_response(&ctx.http, |resp| {
                resp.kind(InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|mes| {
                        crate::build_speaker_selector_response(mes, SpeakerSelector::None);
                        mes
                    })
            })
            .await
            .expect("Failed to create response"),
        _ => unreachable!(),
    }
}
