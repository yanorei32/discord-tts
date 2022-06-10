#![deny(clippy::all)]
#![warn(clippy::pedantic, clippy::nursery)]

mod command;
mod model;
mod listener;
mod persistence;

use std::collections::HashMap;
use std::sync::Mutex;

use once_cell::sync::{Lazy, OnceCell};
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
use crate::persistence::{OnMemorySetting, Persistence};

static CURRENT_TEXT_CHANNEL: Lazy<Mutex<HashMap<GuildId, ChannelId>>> = Lazy::new(|| Mutex::new(HashMap::new()));
static CONFIG: OnceCell<Config> = OnceCell::new();
static ON_MEMORY_SETTING: OnceCell<Mutex<OnMemorySetting>> = OnceCell::new();

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let cfg = envy::from_env::<Config>().expect("Failed to get environment");
    CONFIG
        .set(cfg.clone())
        .unwrap();

    ON_MEMORY_SETTING.set(
        Mutex::new(
            Persistence::load(&cfg).expect("failed to load state from persistence")
        )
    ).unwrap();

    let framework = StandardFramework::new()
        .configure(|c| c.prefix("~"))
        .group(&GENERAL_GROUP);

    let mut client = Client::builder(&cfg.discord_token)
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
