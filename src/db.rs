use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::RwLock;

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
