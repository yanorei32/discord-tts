use crate::{Config, State};
use std::error::Error;
use std::fs::File;
use std::io::{BufReader, Write};

#[derive(Debug)]
pub struct OnMemorySetting {
    pub state: State,
}

impl OnMemorySetting {
    pub fn save(&self, c: &Config) {
        let mut f = File::create(&c.state_path).expect("Unable to open file.");

        let s = &self.state;
        f.write_all(
            serde_json::to_string(&s.user_settings)
                .expect("Failed to serialize")
                .as_bytes(),
        )
        .expect("Unable to write data");
    }
}

#[derive(Debug)]
pub struct Persistence;

impl Persistence {
    pub fn load(config: &Config) -> Result<OnMemorySetting, Box<dyn Error>> {
        let f = match File::open(&config.state_path) {
            Ok(f) => f,
            Err(e) => {
                println!("Failed to read state.json");
                return Err(Box::new(e));
            }
        };

        let reader = BufReader::new(f);
        let res = serde_json::from_reader(reader).expect("JSON was not well-formatted");
        Ok(OnMemorySetting {
            state: State { user_settings: res },
        })
    }
}
