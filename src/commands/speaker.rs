use serenity::{
    all::{ComponentInteraction, ComponentInteractionData, ComponentInteractionDataKind},
    builder::{
        CreateActionRow, CreateAttachment, CreateButton, CreateCommand, CreateEmbed,
        CreateEmbedAuthor, CreateInteractionResponse, CreateInteractionResponseMessage,
        CreateSelectMenu, CreateSelectMenuKind, CreateSelectMenuOption,
    },
    client::Context,
    model::application::CommandInteraction,
};

use crate::voicevox::Client as VoicevoxClient;
use crate::{db::PERSISTENT_DB, voicevox::model::SpeakerId};

pub fn register(prefix: &str) -> CreateCommand {
    CreateCommand::new(format!("{prefix}speaker"))
        .description("Manage your speaker")
        .dm_permission(false)
}

pub async fn run(ctx: &Context, interaction: CommandInteraction, voicevox: &VoicevoxClient) {
    let speaker_id = PERSISTENT_DB.get_speaker_id(interaction.user.id);

    interaction
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(create_modal(voicevox, speaker_id, true)),
        )
        .await
        .unwrap();
}

pub async fn update(ctx: &Context, interaction: ComponentInteraction, voicevox: &VoicevoxClient) {
    let speakers = voicevox.get_speakers();

    let (speaker_id, editable) = match &interaction.data {
        ComponentInteractionData {
            custom_id, kind, ..
        } if custom_id == "speaker_selector" => {
            let ComponentInteractionDataKind::StringSelect { values } = kind else {
                unreachable!("Illegal speaker_selector call");
            };

            let speaker_i: usize = values.first().unwrap().parse().unwrap();

            (speakers[speaker_i].styles.first().unwrap().id, true)
        }
        ComponentInteractionData {
            custom_id, kind, ..
        } if custom_id == "style_selector" => {
            let ComponentInteractionDataKind::StringSelect { values } = kind else {
                unreachable!("Illegal style_selector call");
            };

            (values.first().unwrap().parse().unwrap(), true)
        }
        ComponentInteractionData { custom_id, .. } if custom_id.starts_with("apply_") => {
            let speaker_id: u32 = custom_id.split('_').nth(1).unwrap().parse().unwrap();
            println!("Store {}: {}", interaction.user.id, speaker_id);
            PERSISTENT_DB.store_speaker_id(interaction.user.id, speaker_id);

            (speaker_id, false)
        }
        _ => unimplemented!(),
    };

    interaction
        .create_response(
            &ctx.http,
            CreateInteractionResponse::UpdateMessage(create_modal(voicevox, speaker_id, editable)),
        )
        .await
        .unwrap();
}

pub fn create_modal(
    voicevox: &VoicevoxClient,
    speaker_id: SpeakerId,
    editable: bool,
) -> CreateInteractionResponseMessage {
    let speakers = voicevox.get_speakers();
    let style = voicevox.query_style_by_id(speaker_id).unwrap();

    let core = CreateInteractionResponseMessage::new()
        .embed(
            CreateEmbed::new()
                .author(CreateEmbedAuthor::new(format!(
                    "{} / {}",
                    style.speaker_name, style.style_name
                )))
                .field("Policy", style.speaker_policy, false)
                .thumbnail("attachment://icon.png"),
        )
        .add_file(CreateAttachment::bytes(style.style_icon, "icon.png"))
        .ephemeral(true);

    if !editable {
        return core.components(vec![]);
    }

    core.components(vec![
        CreateActionRow::SelectMenu(CreateSelectMenu::new(
            "speaker_selector",
            CreateSelectMenuKind::String {
                options: speakers
                    .iter()
                    .enumerate()
                    .map(|(i, v)| {
                        CreateSelectMenuOption::new(&v.name, i.to_string())
                            .default_selection(style.speaker_i == i)
                    })
                    .collect(),
            },
        )),
        CreateActionRow::SelectMenu(
            CreateSelectMenu::new(
                "style_selector",
                CreateSelectMenuKind::String {
                    options: speakers[style.speaker_i]
                        .styles
                        .iter()
                        .enumerate()
                        .map(|(i, v)| {
                            CreateSelectMenuOption::new(&v.name, v.id.to_string())
                                .default_selection(style.style_i == i)
                        })
                        .collect(),
                },
            )
            .disabled(speakers[style.speaker_i].styles.len() == 1),
        ),
        CreateActionRow::Buttons(vec![
            CreateButton::new(format!("apply_{speaker_id}")).label("Apply")
        ]),
    ])
}
