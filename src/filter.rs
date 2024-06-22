use std::borrow::Cow;

use once_cell::sync::Lazy;
use regex::Regex;
use serenity::{
    cache::Cache,
    http::CacheHttp,
    model::{channel::Message, id::ChannelId},
    prelude::Mentionable,
};

use crate::db::INMEMORY_DB;

// regex crate's named capture
#[allow(clippy::invalid_regex)]
static CHANNEL_MENTION_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"<#(?<id>\d+)>").unwrap());
static CODEBLOCK_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?sm)```.+```").unwrap());
static EXTERNAL_EMOJI_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"<a?:\w+:\d+>").unwrap());
static EMOJI_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r":\w+:").unwrap());
static URI_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"[A-Za-z][A-Za-z0-9+\-.]*:\S+").unwrap());

pub async fn filter<T>(ctx: T, mes: &'_ Message) -> Option<String>
where
    T: CacheHttp + AsRef<Cache>,
{
    if mes.channel_id != INMEMORY_DB.get_instance(mes.guild_id?)? {
        return None;
    }

    if mes.author.bot {
        return None;
    }

    let s = sanity_mention(ctx, mes).await;
    let s = legacy_command_compatibility(&s)?;
    let s = legacy_ping_command_compatibility(s)?;
    let s = suppress_by_semicolon(s)?;

    // `<a:emoji_identifier:123456789>` should be treated as a single emoji and not `<a:emoji_。URI省略。>`,
    // so replace_external_emoji must precede replace_uri.
    // On the other hand, `protocol:host:23` should be treated as a `。URI省略。` and not `protocol23` (:host: replaced by `replace_emoji`),
    // so replace_uri must precede replace_emoji.
    //
    // The order `replace_external_emoji` -> `replace_uri` -> `replace_emoji` rests upon the observation that
    // an external_emoji cannot be a part of an URI (since an URI cannot contain a letter "<", as per RFC3986),
    // and the design decision that we want to treat a string like `<a:crime:1238318711>` as a single `external_emoji` and not
    // `<。URI省略。>` or `<a:。URI省略。>`. I mean, why would anyone enclose a strange URI within a pair of angle brackets?
    let s = replace_external_emoji(s);
    let s = replace_uri(&s);
    let s = replace_emoji(&s);
    // Attachment::dimensions: If this attachment is an image, then a tuple of the width and height in pixels is returned.
    let s = append_image_attachment_notification(&s, mes.attachments.iter().filter_map(|a| a.dimensions()).count());

    let s = replace_codeblock(&s);
    let s = suppress_whitespaces(&s)?;

    Some(s.to_string())
}

fn append_image_attachment_notification(body: &str, image_count: usize) -> Cow<'_, str> {
    if image_count > 0 {
        const IMAGE_WITH_COMMAS: &str = "画像、";
        const SENT_ON_BLANK: &str = "画像が送信されました";
        // 一応pre-allocate
        let mut ret = String::with_capacity(body.len() + 1 + (IMAGE_WITH_COMMAS.len()) * (image_count - 1) + SENT_ON_BLANK.len());
        ret.push_str(body);
        if !body.is_empty() {
            ret.push(' ');
        }

        for _ in 0..(image_count - 1) {
            ret.push_str(IMAGE_WITH_COMMAS);
        }

        if body.is_empty() {
            ret.push_str(SENT_ON_BLANK);
        } else {
            ret.push_str("画像添付");
        }

        ret.into()
    } else {
        body.into()
    }
}

async fn sanity_mention<T>(ctx: T, mes: &Message) -> String
where
    T: CacheHttp + AsRef<Cache>,
{
    let mut s = mes.content.to_string();

    let guild = mes.guild(&ctx.cache().unwrap()).unwrap();

    for m in &mes.mentions {
        let name = guild
            .members
            .get(&m.id)
            .unwrap()
            .nick
            .as_ref()
            .unwrap_or(m.global_name.as_ref().unwrap_or(&m.name));

        s = s.replace(&m.id.mention().to_string(), &format!("。宛、{name}。"));
    }

    for m in &mes.mention_roles {
        let name = guild.roles.get(&m).unwrap().name.as_str();

        s = s.replace(&m.mention().to_string(), &format!("。宛、{name}。"));
    }

    let channel_mentions: Vec<ChannelId> = CHANNEL_MENTION_REGEX
        .captures_iter(&s)
        .map(|cap| cap.name("id").unwrap().as_str())
        .map(|s| s.parse::<u64>().unwrap().into())
        .collect();

    for m in &channel_mentions {
        let name = guild.channels.get(&m).unwrap().name();
        s = s.replace(&m.mention().to_string(), &format!("。宛、{name}。"));
    }

    s
}

#[inline]
fn legacy_command_compatibility(mes: &str) -> Option<&str> {
    (!mes.starts_with('~')).then_some(mes)
}

#[inline]
fn legacy_ping_command_compatibility(mes: &str) -> Option<&str> {
    (mes != "ping").then_some(mes)
}

#[inline]
fn suppress_by_semicolon(mes: &str) -> Option<&str> {
    (!mes.starts_with(';') || mes.starts_with(";;")).then_some(mes)
}

#[inline]
fn suppress_whitespaces(mes: &str) -> Option<&str> {
    (!mes.trim().is_empty()).then_some(mes)
}

#[inline]
fn replace_uri(mes: &str) -> Cow<'_, str> {
    URI_REGEX.replace_all(mes, "。URI省略。")
}

#[inline]
fn replace_external_emoji(mes: &str) -> Cow<'_, str> {
    EXTERNAL_EMOJI_REGEX.replace_all(mes, "")
}

#[inline]
fn replace_emoji(mes: &str) -> Cow<'_, str> {
    EMOJI_REGEX.replace_all(mes, "")
}

#[inline]
fn replace_codeblock(mes: &str) -> Cow<'_, str> {
    CODEBLOCK_REGEX.replace_all(mes, "。コード省略。")
}

#[test]
fn replace_rule_unit_test() {
    assert_eq!(legacy_command_compatibility("~join"), None);
    assert_eq!(legacy_command_compatibility("hello"), Some("hello"));

    assert_eq!(legacy_ping_command_compatibility("ping"), None);
    assert_eq!(legacy_ping_command_compatibility("hello"), Some("hello"));

    assert_eq!(suppress_by_semicolon("hello"), Some("hello"));
    assert_eq!(suppress_by_semicolon(";hello"), None);
    assert_eq!(suppress_by_semicolon(";;hello"), Some(";;hello"));

    assert_eq!(replace_uri("hello"), "hello");
    assert_eq!(replace_uri("ms-settings:privacy-microphone"), "。URI省略。");
    assert_eq!(
        replace_uri("some.strange-protocol+ver2:pathpathpath"),
        "。URI省略。"
    );
    assert_eq!(
        replace_uri("20:40に秋葉原にて待つ"),
        "20:40に秋葉原にて待つ"
    );
    assert_eq!(replace_uri("abc,def://nyan.com:22/mofu"), "abc,。URI省略。");
    assert_eq!(
        replace_uri("そこから ms-settings:privacy-microphone を開いて"),
        "そこから 。URI省略。 を開いて"
    );
    assert_eq!(
        replace_uri("そこから http://metaba.su を開いて"),
        "そこから 。URI省略。 を開いて"
    );

    assert_eq!(replace_emoji("hello!"), "hello!");
    assert_eq!(replace_emoji("hello:emoji:!"), "hello!");
    assert_eq!(
        replace_external_emoji("hello<:emoji:012345678901234567>!"),
        "hello!"
    );

    assert_eq!(
        replace_codeblock("Codeblock ```Inline``` !"),
        "Codeblock 。コード省略。 !"
    );
    assert_eq!(
        replace_codeblock("Codeblock\n```\nMultiline\n```\n!"),
        "Codeblock\n。コード省略。\n!"
    );
    assert_eq!(
        append_image_attachment_notification("", 0),
        ""
    );
    assert_eq!(
        append_image_attachment_notification("", 1),
        "画像が送信されました"
    );
    assert_eq!(
        append_image_attachment_notification("", 4),
        "画像、画像、画像、画像が送信されました"
    );
    assert_eq!(
        append_image_attachment_notification("あ", 0),
        "あ"
    );
    assert_eq!(
        append_image_attachment_notification("あ", 1),
        "あ 画像添付"
    );
    assert_eq!(
        append_image_attachment_notification("あ", 4),
        "あ 画像、画像、画像、画像添付"
    );
}
