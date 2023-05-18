use serenity::{
    builder::{CreateApplicationCommand, CreateInteractionResponseData},
    client::Context,
    model::{
        application::{
            command::CommandOptionType,
            interaction::{
                application_command::ApplicationCommandInteraction, InteractionResponseType,
            },
        },
        channel::AttachmentType,
        prelude::{
            interaction::{message_component::MessageComponentInteraction, MessageFlags},
            UserId,
        },
    },
};

use crate::db::PERSISTENT_DB;
use crate::interactive_component::{CompileWithBuilder, SelectorResponse};
use crate::model::SpeakerSelector;
use crate::voicevox;

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
            .create_interaction_response(&ctx.http, |resp| {
                resp.kind(InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|mes| {
                        build_current_speaker_response(mes, interaction.user.id);
                        mes.flags(MessageFlags::EPHEMERAL)
                    })
            })
            .await
            .expect("Failed to create response"),
        "change" => interaction
            .create_interaction_response(&ctx.http, |resp| {
                resp.kind(InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|mes| {
                        build_speaker_selector_response(mes, SpeakerSelector::None);
                        mes.flags(MessageFlags::EPHEMERAL)
                    })
            })
            .await
            .expect("Failed to create response"),
        _ => unreachable!(),
    }
}

pub async fn update(ctx: &Context, interaction: MessageComponentInteraction) {
    if interaction.data.custom_id.starts_with("select_style") {
        interaction
            .create_interaction_response(&ctx.http, |response| {
                let id = interaction.data.custom_id.rsplit_once('_').unwrap().1;
                PERSISTENT_DB.store_speaker_id(interaction.user.id, id.parse().unwrap());

                response
                    .kind(InteractionResponseType::UpdateMessage)
                    .interaction_response_data(|message| {
                        build_current_speaker_response(message, interaction.user.id);
                        message
                    })
            })
            .await
            .expect("Failed to create response");
    } else if interaction.data.custom_id.starts_with("speaker") {
        interaction
            .create_interaction_response(&ctx.http, |response| {
                let values = &interaction.data.values;
                let index: usize = values.get(0).unwrap().parse().unwrap();

                response
                    .kind(InteractionResponseType::UpdateMessage)
                    .interaction_response_data(|message| {
                        build_speaker_selector_response(
                            message,
                            SpeakerSelector::SpeakerOnly { speaker: index },
                        );
                        message
                    })
            })
            .await
            .expect("Failed to create response");
    } else if interaction.data.custom_id.starts_with("style") {
        interaction
            .create_interaction_response(&ctx.http, |response| {
                let values = &interaction.data.values;
                let indices: Vec<&str> = values.get(0).unwrap().split('_').collect();
                let speaker_index: usize = indices.first().unwrap().parse().unwrap();
                let style_index: usize = indices.get(1).unwrap().parse().unwrap();

                response
                    .kind(InteractionResponseType::UpdateMessage)
                    .interaction_response_data(|message| {
                        build_speaker_selector_response(
                            message,
                            SpeakerSelector::SpeakerAndStyle {
                                speaker: speaker_index,
                                style: style_index,
                            },
                        );
                        message
                    })
            })
            .await
            .expect("Failed to create response");
    }
}

fn build_current_speaker_response(message: &mut CreateInteractionResponseData, user_id: UserId) {
    let speaker_id = PERSISTENT_DB.get_speaker_id(user_id);
    let speakers = voicevox::get_speakers();

    let (name, style) = speakers
        .iter()
        .flat_map(|speaker| speaker.styles.iter().map(|style| (&speaker.name, style)))
        .find(|v| v.1.id == u32::from(speaker_id))
        .unwrap();

    message
        .add_file(AttachmentType::Bytes {
            data: style.icon.clone(),
            filename: "icon.png".to_string(),
        })
        .embed(|embed| {
            embed
                .author(|author| author.name("Speaker currently in use"))
                .thumbnail("attachment://icon.png")
                .fields([
                    ("Speaker name", &name.to_string(), false),
                    ("Style", &style.name, true),
                    ("id", &style.id.to_string(), true),
                ])
        });
}

fn build_speaker_selector_response(
    message: &mut CreateInteractionResponseData,
    selector: SpeakerSelector,
) {
    let speakers = voicevox::get_speakers();

    let message = match selector {
        SpeakerSelector::SpeakerAndStyle {
            speaker: speaker_index,
            style,
        } => {
            let speaker = speakers.get(speaker_index).unwrap();
            let style = speaker.styles.get(style).unwrap();

            message.add_file(AttachmentType::Bytes {
                data: style.icon.clone(),
                filename: "thumbnail.png".to_string(),
            });

            style
                .samples
                .iter()
                .enumerate()
                .fold(message, |m, (i, sample)| {
                    m.add_file(AttachmentType::Bytes {
                        data: sample.clone(),
                        filename: format!("sample{i}.wav"),
                    })
                })
        }
        SpeakerSelector::SpeakerOnly { speaker: index } => {
            let speaker = speakers.get(index).unwrap();

            message.add_file(AttachmentType::Bytes {
                data: speaker.portrait.clone(),
                filename: "thumbnail.png".to_string(),
            })
        }
        SpeakerSelector::None => message,
    };

    if let Some(speaker_index) = selector.speaker() {
        let speaker = speakers.get(speaker_index).unwrap();

        message.embed(|embed| {
            embed
                .author(|author| author.name("Select speaker you want to use"))
                .thumbnail("attachment://thumbnail.png")
                .field("Name", &speaker.name, true);

            let style = selector.style().map(|a| speaker.styles.get(a).unwrap());
            embed.fields([
                (
                    "Style",
                    style.map_or_else(|| "-".to_string(), |s| s.name.clone()),
                    true,
                ),
                (
                    "ID",
                    style.map_or_else(|| "-".to_string(), |s| s.id.to_string()),
                    true,
                ),
                ("Policy", speaker.policy.clone(), false),
            ])
        });
    }

    SelectorResponse::default().build((speakers, selector), message);
}
