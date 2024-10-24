use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serenity::model::prelude::{ChannelId, GuildId, UserId};

use crate::voicevox::model::SpeakerId;

pub static PERSISTENT_DB: Lazy<PersistentDB> = Lazy::new(|| {
    PersistentDB::new(&crate::config::CONFIG.persistent_path).expect("Failed to initialize DB")
});

#[derive(Serialize, Deserialize, Debug, Clone)]
struct PersistentStructure {
    voice_settings: HashMap<UserId, SpeakerId>,
    speed_settings: HashMap<UserId, f32>,
    speed_default_settings: HashMap<GuildId, f32>,
}

pub struct PersistentDB {
    file: PathBuf,
    data: RwLock<PersistentStructure>,
}

impl PersistentDB {
    fn new(file: &Path) -> Result<Self, std::io::Error> {
        let data =
            serde_json::from_reader(BufReader::new(File::open(file)?)).expect("DB is corrupt");

        Ok(Self {
            file: file.into(),
            data,
        })
    }

    pub fn get_speaker_id(&self, user: UserId) -> SpeakerId {
        self.data
            .read()
            .unwrap()
            .voice_settings
            .get(&user)
            .unwrap_or(&0)
            .to_owned()
    }

    pub fn store_speaker_id(&self, user: UserId, speaker_id: SpeakerId) {
        self.data
            .write()
            .unwrap()
            .voice_settings
            .insert(user, speaker_id);

        self.flush();
    }

    pub fn get_speed(&self, user: UserId, guild: GuildId) -> f32 {
        let default_speed = self.get_speed_default(guild);
        return self
            .data
            .read()
            .unwrap()
            .speed_settings
            .get(&user)
            .unwrap_or(&default_speed)
            .to_owned();
    }

    pub fn store_speed(&self, user: UserId, speed: f32) {
        self.data
            .write()
            .unwrap()
            .speed_settings
            .insert(user, speed);

        self.flush();
    }

    pub fn get_speed_default(&self, guild: GuildId) -> f32 {
        self.data
            .read()
            .unwrap()
            .speed_default_settings
            .get(&guild)
            .unwrap_or(&1.0)
            .to_owned()
    }

    pub fn store_speed_default(&self, guild: GuildId, speed: f32) {
        self.data
            .write()
            .unwrap()
            .speed_default_settings
            .insert(guild, speed);

        self.flush();
    }

    fn flush(&self) {
        File::create(&self.file)
            .expect("Failed to create renew file.")
            .write_all(
                serde_json::to_string(&(*self.data.read().unwrap()))
                    .unwrap()
                    .as_bytes(),
            )
            .expect("Failed to write file.");
    }
}

struct InmemoryStructure {
    instances: HashMap<GuildId, ChannelId>,
}

pub struct InmemoryDB {
    data: RwLock<InmemoryStructure>,
}

pub static INMEMORY_DB: Lazy<InmemoryDB> = Lazy::new(InmemoryDB::new);

impl InmemoryDB {
    fn new() -> Self {
        Self {
            data: RwLock::new(InmemoryStructure {
                instances: HashMap::new(),
            }),
        }
    }

    pub fn get_instance(&self, guild_id: GuildId) -> Option<ChannelId> {
        self.data
            .read()
            .unwrap()
            .instances
            .get(&guild_id)
            .map(ToOwned::to_owned)
    }

    pub fn store_instance(&self, guild_id: GuildId, channel_id: ChannelId) {
        self.data
            .write()
            .unwrap()
            .instances
            .insert(guild_id, channel_id);
    }

    pub fn destroy_instance(&self, guild_id: GuildId) {
        self.data.write().unwrap().instances.remove(&guild_id);
    }
}

pub static EMOJI_DB: Lazy<EmojiDB> = Lazy::new(|| {
    EmojiDB::new().expect("Failed to initialize emoji DB")
});

#[derive(Serialize, Deserialize, Debug, Clone)]
struct EmojiStructure {
    short_name: String,
}

pub struct EmojiDB {
    data: Arc<HashMap<String, String>>,
}

impl EmojiDB {
    fn new() -> Result<Self, std::io::Error> {
        let json: HashMap<String, EmojiStructure> =
            serde_json::from_str(include_str!("../assets/emoji_ja.json"))
                .expect("Emoji DB is corrupted");

        let data = Arc::new(
            json.iter()
                .map(|(key, value)| (key.clone(), value.short_name.clone()))
                .collect(),
        );

        Ok(Self {
            data,
        })
    }

    pub fn get_dictionary(&self) -> Arc<HashMap<String, String>> {
        self.data.clone()
    }
}
