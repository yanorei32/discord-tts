use std::marker::PhantomData;
use serenity::builder::{CreateButton, CreateComponents, CreateInteractionResponseData, CreateSelectMenu};
use serenity::model::application::component::ButtonStyle;
use tap::Pipe;
use crate::SpeakerSelector;
use crate::voicevox::model::Speaker;

/// コンポーネントの依存性を分離して親子関係を明確にするためのトレイト
pub trait CompileWithBuilder {
    /// パラメーター。
    /// ない場合は`()`を設定せよ。
    /// 複数ある場合はタプルにするか、構造体にせよ。
    type Parameters: Sized;
    /// 親のビルダー。
    type ParentBuilder: Sized;
    fn build(self, params: Self::Parameters, parent: &mut Self::ParentBuilder) -> &mut Self::ParentBuilder;
}

macro_rules! rows {
    ($cc:ident, $param:expr, $x:ident, $($xs: ident),+) => {{
        let $cc = $x::default().pipe(GenericRow).build($param.clone(), $cc);
        rows!($cc, $param, $($xs),+)
    }};
    ($cc:ident, $param:expr, $x:ident) => {{
        let $cc = $x::default().pipe(GenericRow).build($param.clone(), $cc);
        $cc
    }};
}

#[derive(Default)]
pub struct SelectorResponse<'a>(PhantomData<fn() -> &'a ()>);
impl<'a> CompileWithBuilder for SelectorResponse<'a> {
    type Parameters = (Vec<Speaker<'a>>, SpeakerSelector);
    type ParentBuilder = CreateInteractionResponseData<'a>;

    fn build(self, t: Self::Parameters, parent: &mut Self::ParentBuilder) -> &mut Self::ParentBuilder {
        parent.components(|c| {
            rows!(c, t, SpeakerSelectionMenu, StyleSelectionMenu, ApplyStyleButton)
        })
    }
}

#[derive(Default)]
struct SpeakerSelectionMenu<'a>(PhantomData<fn() -> &'a ()>);
impl<'a> CompileWithBuilder for SpeakerSelectionMenu<'a> {
    type Parameters = (Vec<Speaker<'a>>, SpeakerSelector);
    type ParentBuilder = CreateSelectMenu;

    fn build(self, (speakers, selector): Self::Parameters, parent: &mut Self::ParentBuilder) -> &mut Self::ParentBuilder {
        parent
            .placeholder("Speaker selection")
            .custom_id("speaker")
            .options(|options| {
                for (i, speaker) in speakers.iter().enumerate() {
                    options.create_option(|option| {
                        option
                            .description("")
                            .label(&speaker.name)
                            .value(i)
                            .default_selection(selector.speaker() == Some(i))
                    });
                }
                options
            })
    }
}

#[derive(Default)]
struct StyleSelectionMenu<'a>(PhantomData<fn() -> &'a ()>);
impl<'a> CompileWithBuilder for StyleSelectionMenu<'a> {
    type Parameters = (Vec<Speaker<'a>>, SpeakerSelector);
    type ParentBuilder = CreateSelectMenu;

    fn build(self, (speakers, selector): Self::Parameters, menu: &mut Self::ParentBuilder) -> &mut Self::ParentBuilder {
        menu.placeholder("Style selection")
            .custom_id("style")
            .options(|options| {
                // E0524回避のためrevert
                if let Some(index) = selector.speaker() {
                    let speaker = speakers.get(index).unwrap();

                    speaker.styles.iter().enumerate().fold(options, |opts, (i, style)| {
                        opts.create_option(|option| {
                            option
                                .description("")
                                .label(&style.name)
                                .value(format!("{}_{}", index, i))
                                .default_selection(selector.style() == Some(i))
                        })
                    })
                } else {
                    options.create_option(|option| {
                        option
                            .description("")
                            .label("No options found")
                            .value("disabled")
                    })
                }
            })
            .disabled(selector.speaker().is_none())
    }
}

#[derive(Default)]
struct ApplyStyleButton<'a>(PhantomData<fn() -> &'a ()>);
impl<'a> CompileWithBuilder for ApplyStyleButton<'a> {
    type Parameters = (Vec<Speaker<'a>>, SpeakerSelector);
    type ParentBuilder = CreateButton;

    fn build(self, (speakers, selector): Self::Parameters, button: &mut Self::ParentBuilder) -> &mut Self::ParentBuilder {
        button
            .style(ButtonStyle::Success)
            .label("Select this style");

        if let SpeakerSelector::SpeakerAndStyle {
            speaker: speaker_index,
            style: style_index,
        } = selector
        {
            let speaker = speakers.get(speaker_index).unwrap();
            let style = speaker.styles.get(style_index).unwrap();
            button.custom_id(format!("select_style_{}", style.id))
        } else {
            button.custom_id("select_style_disabled").disabled(true)
        }
    }
}

struct GenericRow<T: CompileWithBuilder<Parameters=P,ParentBuilder=CreationBuilder>, P, CreationBuilder>(T);

macro_rules! gen_row {
    ($pb:ty,$m:ident) => {
        impl<T: CompileWithBuilder<Parameters=P,ParentBuilder=$pb>, P> CompileWithBuilder for GenericRow<T, P, $pb> {
            type Parameters = P;
            type ParentBuilder = CreateComponents;

            fn build(self, params: Self::Parameters, parent: &mut Self::ParentBuilder) -> &mut Self::ParentBuilder {
                parent.create_action_row(|r| {
                    r.$m(|m| self.0.build(params, m))
                })
            }
        }
    };
}

gen_row!(CreateSelectMenu, create_select_menu);
gen_row!(CreateButton, create_button);
