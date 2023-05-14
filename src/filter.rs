use std::borrow::Cow;

use once_cell::sync::Lazy;
use regex::Regex;
use serenity::{
    cache::Cache,
    http::CacheHttp,
    model::{channel::Message, id::ChannelId},
    prelude::Mentionable,
};

static CODEBLOCK_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)```.+```").unwrap());
static EMOJI_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"(<:\w+:\d{18}>|:\w+:)").unwrap());
static URI_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"\S+:\S+").unwrap());
static CHANNEL_MENTION_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"<#(?<id>\d{18})>").unwrap());

pub async fn filter<T>(ctx: T, mes: &'_ Message) -> Option<String>
where
    T: CacheHttp + AsRef<Cache>,
{
    if mes.author.bot {
        return None;
    }

    if mes.guild_id.is_none() {
        return None;
    }

    let s = sanity_mention(ctx, mes).await;
    let s = legacy_command_compatibility(&s)?;
    let s = legacy_ping_command_compatibility(s)?;
    let s = suppress_by_semicolon(s)?;
    let s = replace_emoji(s);
    let s = replace_uri(&s);
    let s = replace_codeblock(&s);
    let s = suppress_whitespaces(&s)?;
    Some(s.to_string())
}

async fn sanity_mention<T>(ctx: T, mes: &Message) -> String
where
    T: CacheHttp + AsRef<Cache>,
{
    let mut s = mes.content.to_string();

    for m in &mes.mentions {
        let name = m
            .nick_in(&ctx, mes.guild_id.unwrap())
            .await
            .unwrap_or(m.name.clone());

        s = Regex::new(&m.id.mention().to_string())
            .unwrap()
            .replace_all(&s, format!("。宛、{name}。"))
            .to_string();
    }

    for m in &mes.mention_roles {
        let re = Regex::new(&m.mention().to_string()).unwrap();
        let name = m.to_role_cached(&ctx).unwrap().name;
        s = re
            .replace_all(&s, format!("。宛、{name}。"))
            .to_string();
    }

    let channel_mentions: Vec<ChannelId> = CHANNEL_MENTION_REGEX
        .captures_iter(&s)
        .map(|cap| cap.name("id").unwrap().as_str())
        .map(|s| s.parse::<u64>().unwrap().into())
        .collect();

    for m in &channel_mentions {
        let re = Regex::new(&m.mention().to_string()).unwrap();
        let name = m.name(&ctx).await.unwrap();
        s = re.replace_all(&s, format!("。宛、{name}。")).to_string();
    }

    s
}

fn legacy_command_compatibility(mes: &str) -> Option<&str> {
    (!mes.starts_with('~')).then_some(mes)
}

fn legacy_ping_command_compatibility(mes: &str) -> Option<&str> {
    (mes != "ping").then_some(mes)
}

fn suppress_by_semicolon(mes: &str) -> Option<&str> {
    (!mes.starts_with(';') || mes.starts_with(";;")).then_some(mes)
}

fn suppress_whitespaces(mes: &str) -> Option<&str> {
    (!mes.trim().is_empty()).then_some(mes)
}

fn replace_uri(mes: &str) -> Cow<'_, str> {
    URI_REGEX.replace_all(mes, "。URI省略。")
}

fn replace_emoji(mes: &str) -> Cow<'_, str> {
    EMOJI_REGEX.replace_all(mes, "")
}

fn replace_codeblock(mes: &str) -> Cow<'_, str> {
    CODEBLOCK_REGEX.replace_all(mes, "。コード省略。")
}
