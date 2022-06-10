#![deny(clippy::all)]
#![warn(clippy::pedantic, clippy::nursery)]

mod command;
mod model;
mod listener;

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Write};
use std::sync::Mutex;

use once_cell::sync::OnceCell;
use serenity::{
    client::Client,
    framework::StandardFramework,
    model::{
        channel::Message,
        prelude::{ChannelId, GuildId},
    },
    Result as SerenityResult,
};
use songbird::SerenityInit;
use crate::{
    command::GENERAL_GROUP,
    model::{
        State, Config, UserSettings
    }
};
use crate::listener::serenity::Handler;

#[macro_use]
extern crate lazy_static;

lazy_static! {
    static ref CURRENT_TEXT_CHANNEL: Mutex<HashMap<GuildId, ChannelId>> =
        Mutex::new(HashMap::new());
    static ref STATE: Mutex<State> = Mutex::new(State {
        user_settings: HashMap::new()
    });
}

static CONFIG: OnceCell<Config> = OnceCell::new();

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    CONFIG
        .set(envy::from_env::<Config>().expect("Failed to get environment"))
        .unwrap();

    load_state();

    let framework = StandardFramework::new()
        .configure(|c| c.prefix("~"))
        .group(&GENERAL_GROUP);

    let c = CONFIG.get().unwrap();
    let mut client = Client::builder(&c.discord_token)
        .event_handler(Handler)
        .framework(framework)
        .register_songbird()
        .await
        .expect("Failed to create client");

    tokio::spawn(async move {
        let _ = client
            .start()
            .await
            .map_err(|why| println!("Client ended: {:?}", why));
    });

    tokio::signal::ctrl_c()
        .await
        .expect("Failed to wait Ctrl+C");

    println!("Received Ctrl+C, shutting down.");
}

fn check_msg(result: SerenityResult<Message>) {
    if let Err(why) = result {
        println!("Error sending message: {:?}", why);
    }
}

fn save_state() {
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

fn load_state() {
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
