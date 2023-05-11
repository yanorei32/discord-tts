use once_cell::sync::Lazy;
use regex::Regex;
use std::borrow::Cow;

static CODEBLOCK_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)```.+```").unwrap());
static URI_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"\S+:\S+").unwrap());

pub fn filter(mes: &str) -> Option<String> {
    let mes = legacy_command_compatibility(mes)?;
    let mes = legacy_ping_command_compatibility(mes)?;
    let mes = suppress_by_semicolon(mes)?;
    let mes = replace_uri(mes);
    let mes = replace_codeblock(&mes);
    let mes = suppress_whitespaces(&mes)?;
    Some(mes.to_string())
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

fn replace_codeblock(mes: &str) -> Cow<'_, str> {
    CODEBLOCK_REGEX.replace_all(mes, "。コード省略。")
}
