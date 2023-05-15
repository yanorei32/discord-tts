use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serenity::model::prelude::UserId;

pub static PERSISTENT_DB: Lazy<PersistentDB> = Lazy::new(|| {
    PersistentDB::new(&crate::config::CONFIG.persistent_path).expect("Failed to initialize DB")
});

#[derive(Serialize, Deserialize, Debug, Clone)]
struct PersistentStructure {
    voice_settings: HashMap<UserId, u8>,
}

pub struct PersistentDB {
    file: PathBuf,
    data: RwLock<PersistentStructure>,
}

impl PersistentDB {
    pub fn new(file: &Path) -> Result<Self, std::io::Error> {
        let file = file.into();
        let data =
            serde_json::from_reader(BufReader::new(File::open(&file)?)).expect("DB is corrupt");

        Ok(Self { file, data })
    }

    pub fn get_speaker_id(&self, user: UserId) -> u8 {
        self.data
            .read()
            .unwrap()
            .voice_settings
            .get(&user)
            .unwrap_or(&0)
            .to_owned()
    }

    pub fn store_speaker_id(&self, user: UserId, speaker_id: u8) {
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
