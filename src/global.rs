use once_cell::sync::{Lazy, OnceCell};
use std::sync::Mutex;
use std::collections::HashMap;
use serenity::model::id::{ChannelId, GuildId};
use crate::{Config, OnMemorySetting};

pub static CURRENT_TEXT_CHANNEL: Lazy<Mutex<HashMap<GuildId, ChannelId>>> = Lazy::new(|| Mutex::new(HashMap::new()));
pub static CONFIG: OnceCell<Config> = OnceCell::new();
pub static ON_MEMORY_SETTING: OnceCell<Mutex<OnMemorySetting>> = OnceCell::new();
