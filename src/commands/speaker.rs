use serenity::{
    builder::{CreateApplicationCommand, CreateInteractionResponse},
    client::Context,
    model::{
        application::interaction::{
            application_command::ApplicationCommandInteraction, InteractionResponseType,
        },
        channel::AttachmentType,
        prelude::interaction::message_component::{
            MessageComponentInteraction, MessageComponentInteractionData,
        },
    },
};

use crate::voicevox::Client as VoicevoxClient;
use crate::{db::PERSISTENT_DB, voicevox::model::SpeakerId};

pub fn register(cmd: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    cmd.name("speaker")
        .description("Manage your speaker")
        .dm_permission(false)
}

pub async fn run(
    ctx: &Context,
    interaction: ApplicationCommandInteraction,
    voicevox: &VoicevoxClient,
) {
    let speaker_id = PERSISTENT_DB.get_speaker_id(interaction.user.id);

    interaction
        .create_interaction_response(&ctx.http, |resp| {
            create_modal(resp, voicevox, speaker_id);
            resp
        })
        .await
        .unwrap();
}

pub async fn update(
    ctx: &Context,
    interaction: MessageComponentInteraction,
    voicevox: &VoicevoxClient,
) {
    let speakers = voicevox.get_speakers();
    let speaker_id: SpeakerId = match &interaction.data {
        MessageComponentInteractionData {
            custom_id, values, ..
        } if custom_id == "speaker_selector" => {
            let speaker_i: usize = values.first().unwrap().parse().unwrap();

            speakers[speaker_i].styles.first().unwrap().id
        }
        MessageComponentInteractionData {
            custom_id, values, ..
        } if custom_id == "style_selector" => values.first().unwrap().parse().unwrap(),
        MessageComponentInteractionData { custom_id, .. } if custom_id.starts_with("apply_") => {
            let speaker_id: u32 = custom_id.split('_').nth(1).unwrap().parse().unwrap();
            println!("Store {}: {}", interaction.user.id, speaker_id);
            PERSISTENT_DB.store_speaker_id(interaction.user.id, speaker_id);

            interaction
                .create_interaction_response(&ctx.http, |response| {
                    response
                        .kind(InteractionResponseType::UpdateMessage)
                        .interaction_response_data(|mes| {
                            mes.components(|comp| comp.set_action_rows(Vec::new()))
                        })
                })
                .await
                .unwrap();

            return;
        }
        _ => unimplemented!(),
    };

    interaction
        .create_interaction_response(&ctx.http, |resp| {
            create_modal(resp, voicevox, speaker_id);
            resp.kind(InteractionResponseType::UpdateMessage)
        })
        .await
        .unwrap();
}

pub fn create_modal<'a>(
    resp: &mut CreateInteractionResponse<'a>,
    voicevox: &'a VoicevoxClient,
    speaker_id: SpeakerId,
) {
    let speakers = voicevox.get_speakers();
    let style = voicevox.query_style_by_id(speaker_id).unwrap();

    resp.kind(InteractionResponseType::ChannelMessageWithSource)
        .interaction_response_data(|mes| {
            mes.embed(|embed| {
                embed
                    .author(|author| {
                        author.name(format!("{} / {}", style.speaker_name, style.style_name))
                    })
                    .field("Policy", style.speaker_policy, false)
                    .thumbnail("attachment://icon.png")
            })
            .add_file(AttachmentType::Bytes {
                data: style.style_icon,
                filename: "icon.png".to_string(),
            })
            .components(|comp| {
                comp.create_action_row(|v| {
                    v.create_select_menu(|sel| {
                        sel.options(|opts| {
                            speakers.iter().enumerate().fold(opts, |opts, (i, v)| {
                                opts.create_option(|opt| {
                                    opt.default_selection(style.speaker_i == i)
                                        .label(&v.name)
                                        .value(i)
                                })
                            })
                        })
                        .custom_id("speaker_selector")
                    })
                })
                .create_action_row(|v| {
                    v.create_select_menu(|sel| {
                        sel.options(|opts| {
                            speakers[style.speaker_i].styles.iter().enumerate().fold(
                                opts,
                                |opts, (i, v)| {
                                    opts.create_option(|opt| {
                                        opt.default_selection(style.style_i == i)
                                            .label(&v.name)
                                            .value(v.id)
                                    })
                                },
                            )
                        })
                        .custom_id("style_selector")
                    })
                })
                .create_action_row(|v| {
                    v.create_button(|btn| {
                        btn.label("Apply").custom_id(format!("apply_{speaker_id}"))
                    })
                })
            })
            .ephemeral(true)
        });
}
