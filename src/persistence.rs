use std::fs::File;
use std::io::{BufReader, Write};
use crate::{CONFIG, STATE};

struct Persistence;

impl Persistence {
    fn save() {
        let c = CONFIG.get().unwrap();
        let mut f = File::create(&c.state_path).expect("Unable to open file.");

        let s = STATE.lock().unwrap();
        f.write_all(
            serde_json::to_string(&s.user_settings)
                .expect("Failed to serialize")
                .as_bytes(),
        )
            .expect("Unable to write data");
    }

    fn load() {
        let c = CONFIG.get().unwrap();
        match File::open(&c.state_path) {
            Ok(f) => {
                let reader = BufReader::new(f);
                let mut s = STATE.lock().unwrap();
                s.user_settings = serde_json::from_reader(reader).expect("JSON was not well-formatted");
            }
            Err(_) => {
                println!("Failed to read state.json");
            }
        }
    }
}
