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
use crate::{db::PERSISTENT_DB, voicevox::model::StyleId};

const PAGE_SIZE: usize = 25;

pub fn register(prefix: &str) -> CreateCommand {
    CreateCommand::new(format!("{prefix}speaker"))
        .description("Manage your speaker")
        .dm_permission(false)
}

pub async fn run(ctx: &Context, interaction: CommandInteraction, voicevox: &VoicevoxClient) {
    let style_id = PERSISTENT_DB
        .get_style_id(interaction.user.id)
        .unwrap_or(voicevox.default_style());

    interaction
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(create_modal(voicevox, style_id, true)),
        )
        .await
        .unwrap();
}

pub async fn update(ctx: &Context, interaction: ComponentInteraction, voicevox: &VoicevoxClient) {
    let speakers = voicevox.get_speakers();

    let (style_id, editable) = match &interaction.data {
        ComponentInteractionData {
            custom_id, kind, ..
        } if custom_id == "speaker_page_selector" => {
            let ComponentInteractionDataKind::StringSelect { values } = kind else {
                unreachable!("Illegal speaker_page_selector call");
            };

            let page_index: usize = values.first().unwrap().parse().unwrap();

            (
                speakers[page_index * PAGE_SIZE].styles.first().unwrap().id,
                true,
            )
        }
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
            let style_id = custom_id.split('_').nth(1).unwrap().parse().unwrap();
            println!("Store {}: {}", interaction.user.id, style_id);
            PERSISTENT_DB.store_style_id(interaction.user.id, style_id);

            (style_id, false)
        }
        _ => unimplemented!(),
    };

    interaction
        .create_response(
            &ctx.http,
            CreateInteractionResponse::UpdateMessage(create_modal(voicevox, style_id, editable)),
        )
        .await
        .unwrap();
}

pub fn create_modal(
    voicevox: &VoicevoxClient,
    style_id: StyleId,
    editable: bool,
) -> CreateInteractionResponseMessage {
    let speakers = voicevox.get_speakers();

    let style = voicevox.query_style_by_id(style_id).unwrap_or(
        voicevox
            .query_style_by_id(voicevox.default_style())
            .unwrap(),
    );

    let paged_speakers: Vec<_> = voicevox.get_speakers().chunks(PAGE_SIZE).collect();
    let page_count = (speakers.len() + PAGE_SIZE - 1) / PAGE_SIZE;
    let current_page_index = style.speaker_i / PAGE_SIZE;

    let policy = match style.speaker_policy {
        policy if policy.starts_with("# Aivis Common Model License (ACML) 1.0\n") =>
            "この音声は [Aivis Common Model License (ACML) 1.0](https://github.com/Aivis-Project/ACML/blob/master/ACML-1.0.md) により提供されています。".to_string(),
        policy if policy.starts_with("# Aivis Common Model License (ACML) - Non Commercial 1.0\n") =>
            "この音声は [Aivis Common Model License (ACML) - Non Commercial 1.0](https://github.com/Aivis-Project/ACML/blob/master/ACML-NC-1.0.md) により提供されています。".to_string(),
        policy =>
            policy.chars().take(512).collect::<String>(),
    };

    let core = CreateInteractionResponseMessage::new()
        .embed(
            CreateEmbed::new()
                .author(CreateEmbedAuthor::new(format!(
                    "{} / {}",
                    style.speaker_name, style.style_name
                )))
                .field("Policy", policy, false)
                .thumbnail("attachment://icon.png"),
        )
        .add_file(CreateAttachment::bytes(style.style_icon, "icon.png"))
        .ephemeral(true);

    if !editable {
        return core.components(vec![]);
    }

    core.components(vec![
        CreateActionRow::SelectMenu(
            CreateSelectMenu::new(
                "speaker_page_selector",
                CreateSelectMenuKind::String {
                    options: (0..page_count)
                        .map(|i| {
                            CreateSelectMenuOption::new(
                                &format!("Page {}/{}", i + 1, page_count),
                                i.to_string(),
                            )
                            .default_selection(current_page_index == i)
                        })
                        .collect(),
                },
            )
            .disabled(page_count == 1),
        ),
        CreateActionRow::SelectMenu(CreateSelectMenu::new(
            "speaker_selector",
            CreateSelectMenuKind::String {
                options: paged_speakers[current_page_index]
                    .iter()
                    .enumerate()
                    .map(|(i, v)| {
                        CreateSelectMenuOption::new(
                            &v.name,
                            (current_page_index * PAGE_SIZE + i).to_string(),
                        )
                        .default_selection(style.speaker_i == (current_page_index * PAGE_SIZE + i))
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
            CreateButton::new(format!("apply_{style_id}")).label("Apply")
        ]),
    ])
}
