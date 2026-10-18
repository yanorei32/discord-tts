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

use crate::{DEFAULT_TTS_STYLE, db::PERSISTENT_DB};
use tts_bot_core::{model::TtsStyle, tts::TtsServices};

const PAGE_SIZE: usize = 25;

pub fn register(prefix: &str) -> CreateCommand {
    CreateCommand::new(format!("{prefix}speaker"))
        .description("Manage your speaker")
        .dm_permission(false)
}

pub async fn run(ctx: &Context, interaction: CommandInteraction, tts_services: &TtsServices) {
    let voice_setting = PERSISTENT_DB
        .get_voice_setting(interaction.user.id)
        .unwrap_or(DEFAULT_TTS_STYLE.get().unwrap().clone());

    interaction
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                create_modal(tts_services, &voice_setting, true).await,
            ),
        )
        .await
        .unwrap();
}

fn encode(service: &str, style: &str, g2p: Option<&str>) -> String {
    match g2p {
        Some(id) => format!("{service}_!DISCORDTTS!_{style}_!G2P!_{id}"),
        None => format!("{service}_!DISCORDTTS!_{style}"),
    }
}

fn parse_tts_style(s: &str) -> TtsStyle {
    let (head, g2p) = match s.split_once("_!G2P!_") {
        Some((head, g2p)) => (head, Some(g2p.to_string())),
        None => (s, None),
    };
    let (service_id, style_id) = head
        .split_once("_!DISCORDTTS!_")
        .expect("UNKNOWN TTS TYLE FORMAT");
    TtsStyle {
        service_id: service_id.to_string(),
        style_id: style_id.to_string(),
        g2p,
    }
}

pub async fn update(ctx: &Context, interaction: ComponentInteraction, tts_services: &TtsServices) {
    let (style, editable) = match &interaction.data {
        ComponentInteractionData {
            custom_id, kind, ..
        } if custom_id == "page_selector"
            || custom_id == "character_selector"
            || custom_id == "style_selector"
            || custom_id == "g2p_selector" =>
        {
            let ComponentInteractionDataKind::StringSelect { values } = kind else {
                unreachable!("Illegal style_selector call");
            };

            (parse_tts_style(values.first().unwrap()), true)
        }
        ComponentInteractionData { custom_id, .. } if custom_id.starts_with("apply_") => {
            let (_apply, style) = custom_id.split_once('_').unwrap();
            let style = parse_tts_style(style);
            PERSISTENT_DB.store_style_id(interaction.user.id, &style);

            (style, false)
        }
        _ => unimplemented!(),
    };

    interaction
        .create_response(
            &ctx.http,
            CreateInteractionResponse::UpdateMessage(
                create_modal(tts_services, &style, editable).await,
            ),
        )
        .await
        .unwrap();
}

#[allow(clippy::too_many_lines)]
pub async fn create_modal(
    tts_services: &TtsServices,
    voice_setting: &TtsStyle,
    editable: bool,
) -> CreateInteractionResponseMessage {
    let styles = tts_services.styles().await;

    // Check avialablity
    let voice_setting = if tts_services
        .is_available(&voice_setting.service_id, &voice_setting.style_id)
        .await
    {
        voice_setting
    } else {
        DEFAULT_TTS_STYLE.get().unwrap()
    };

    let current_service = styles.get(&voice_setting.service_id).unwrap();

    let current_speaker = current_service
        .iter()
        .find(|character| {
            character
                .styles
                .iter()
                .any(|style| style.id == voice_setting.style_id)
        })
        .unwrap();

    let current_style = current_speaker
        .styles
        .iter()
        .find(|style| style.id == voice_setting.style_id)
        .unwrap();

    let g2p_matches = tts_services
        .g2p_choices(&voice_setting.service_id, &voice_setting.style_id)
        .await;

    // 保存された選択が使えない場合は、そのスタイルで使える最初の G2P をデフォルトにする。
    let active_g2p = match voice_setting.g2p {
        Some(ref id) if g2p_matches.iter().any(|(engine_id, _)| engine_id == id) => {
            Some(id.as_ref())
        }
        Some(_) => g2p_matches.first().map(|(engine_id, _)| engine_id.as_str()),
        None => None,
    };

    let mut pages = vec![];

    let mut current_character_items = vec![];

    let mut current_page_id = String::new();

    for (service, characters) in &styles {
        if characters.len() <= PAGE_SIZE {
            let first_style_id = &characters.first().unwrap().styles.first().unwrap().id;
            let g2p = tts_services.g2p_choices(service, first_style_id).await;
            let g2p = g2p.first().map(|(id, _)| id.as_ref());
            let transition_target_id = encode(service, first_style_id, g2p);

            pages.push((service.clone(), transition_target_id.clone()));

            if service == &voice_setting.service_id {
                current_page_id = transition_target_id;
                current_character_items.clone_from(characters);
            }

            continue;
        }

        let page_count = characters.len().div_ceil(PAGE_SIZE);
        for (page_index, page_characters) in characters.chunks(PAGE_SIZE).enumerate() {
            let page_index = page_index + 1;

            let first_style_id = &page_characters.first().unwrap().styles.first().unwrap().id;
            let g2p = tts_services.g2p_choices(service, first_style_id).await;
            let g2p = g2p.first().map(|(id, _)| id.as_ref());
            let transition_target_id = encode(service, first_style_id, g2p);

            pages.push((
                format!("{service} ({page_index}/{page_count})"),
                transition_target_id.clone(),
            ));

            if service == &voice_setting.service_id
                && page_characters
                    .iter()
                    .flat_map(|character| character.styles.iter())
                    .any(|style| style.id == voice_setting.style_id)
            {
                current_page_id = transition_target_id;
                current_character_items = page_characters.to_vec();
            }
        }
    }

    let mut characters = vec![];

    let mut current_style_items = vec![];
    let mut current_character_id = String::new();

    for character in current_character_items {
        let first_style_id = &character.styles.first().unwrap().id;

        let g2p = tts_services
            .g2p_choices(&voice_setting.service_id, first_style_id)
            .await;
        let g2p = g2p.first().map(|(id, _)| id.as_ref());
        let transition_target_id = encode(&voice_setting.service_id, first_style_id, g2p);

        characters.push((character.name.clone(), transition_target_id.clone()));

        if character
            .styles
            .iter()
            .any(|style| style.id == voice_setting.style_id)
        {
            current_character_id = transition_target_id;
            current_style_items = character.styles;
        }
    }

    let mut styles = vec![];
    let mut current_style_id = String::new();

    for style in current_style_items {
        let style_id = &style.id;

        let g2p = tts_services
            .g2p_choices(&voice_setting.service_id, style_id)
            .await;
        let g2p = g2p.first().map(|(id, _)| id.as_ref());
        let transition_target_id = encode(&voice_setting.service_id, style_id, g2p);

        styles.push((style.name.clone(), transition_target_id.clone()));

        if voice_setting.style_id == *style_id {
            current_style_id = transition_target_id;
        }
    }

    let core = CreateInteractionResponseMessage::new()
        .embed(
            CreateEmbed::new()
                .author(CreateEmbedAuthor::new(format!(
                    "{} / {}",
                    current_speaker.name, current_style.name
                )))
                .field("Policy", &current_speaker.policy, false)
                .thumbnail("attachment://icon.png"),
        )
        .add_file(CreateAttachment::bytes(
            current_style.icon.clone(),
            "icon.png",
        ))
        .ephemeral(true);

    if !editable {
        return core.components(vec![]);
    }

    let page_options: Vec<_> = pages
        .into_iter()
        .map(|(display, transition_to)| {
            let is_default = current_page_id == transition_to;
            CreateSelectMenuOption::new(display, transition_to).default_selection(is_default)
        })
        .collect();

    let page_unselectable = page_options.len() <= 1;

    let character_options: Vec<_> = characters
        .into_iter()
        .map(|(display, transition_to)| {
            let is_default = current_character_id == transition_to;
            CreateSelectMenuOption::new(display, transition_to).default_selection(is_default)
        })
        .collect();

    let character_unselectable = character_options.len() <= 1;

    let apply_target_id = encode(
        &voice_setting.service_id,
        &voice_setting.style_id,
        active_g2p,
    );

    let style_options: Vec<_> = styles
        .into_iter()
        .map(|(display, transition_to)| {
            let is_default = current_style_id == transition_to;
            CreateSelectMenuOption::new(display, transition_to).default_selection(is_default)
        })
        .collect();

    let style_unselectable = style_options.len() <= 1;

    let g2p_menu = if g2p_matches.is_empty() {
        None
    } else {
        let mut options = vec![
            CreateSelectMenuOption::new(
                "None",
                encode(&voice_setting.service_id, &voice_setting.style_id, None),
            )
            .default_selection(active_g2p.is_none()),
        ];

        for (id, friendly_name) in &g2p_matches {
            options.push(
                CreateSelectMenuOption::new(
                    friendly_name.as_str(),
                    encode(
                        &voice_setting.service_id,
                        &voice_setting.style_id,
                        Some(id.as_str()),
                    ),
                )
                .default_selection(active_g2p == Some(id.as_str())),
            );
        }
        Some(CreateActionRow::SelectMenu(CreateSelectMenu::new(
            "g2p_selector",
            CreateSelectMenuKind::String { options },
        )))
    };

    let mut rows = vec![
        CreateActionRow::SelectMenu(
            CreateSelectMenu::new(
                "page_selector",
                CreateSelectMenuKind::String {
                    options: page_options,
                },
            )
            .disabled(page_unselectable),
        ),
        CreateActionRow::SelectMenu(
            CreateSelectMenu::new(
                "character_selector",
                CreateSelectMenuKind::String {
                    options: character_options,
                },
            )
            .disabled(character_unselectable),
        ),
        CreateActionRow::SelectMenu(
            CreateSelectMenu::new(
                "style_selector",
                CreateSelectMenuKind::String {
                    options: style_options,
                },
            )
            .disabled(style_unselectable),
        ),
    ];
    if let Some(menu) = g2p_menu {
        rows.push(menu);
    }
    rows.push(CreateActionRow::Buttons(vec![
        CreateButton::new(format!("apply_{apply_target_id}")).label("Apply"),
    ]));
    core.components(rows)
}
