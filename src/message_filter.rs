use regex::Regex;
use std::borrow::Cow;
use once_cell::sync::Lazy;

static CODEBLOCK_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?m)```.+```").unwrap()
});

static URI_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\S+:\S+").unwrap()
});

pub fn filter(mes: &str) -> Option<String> {
    let mes = remove_command_like_string(mes)?;
    let mes = legacy_ping_command_compatibility(mes)?;
    let mes = suppress_by_semicolon(mes)?;
    let mes = replace_uri(mes);
    let mes = replace_codeblock(&mes);
    let mes = suppress_whitespaces(&mes)?;
    Some(mes.to_string())
}

fn remove_command_like_string(mes: &str) -> Option<&str> {
    if mes.get(..1) == Some("~") {
        None
    } else {
        Some(mes)
    }
}

fn legacy_ping_command_compatibility(mes: &str) -> Option<&str> {
    if mes == "ping" {
        None
    } else {
        Some(mes)
    }
}

fn suppress_by_semicolon(mes: &str) -> Option<&str> {
    if !mes.starts_with(";;") && mes.starts_with(';') {
        None
    } else {
        Some(mes)
    }
}

fn suppress_whitespaces(mes: &str) -> Option<&str> {
    if mes.trim().is_empty() {
        None
    } else {
        Some(mes)
    }
}

fn replace_uri(mes: &str) -> Cow<'_, str> {
    URI_REGEX.replace_all(mes, "。URI省略。")
}

fn replace_codeblock(mes: &str) -> Cow<'_, str> {
    CODEBLOCK_REGEX.replace_all(mes, "。コード省略。")
}
