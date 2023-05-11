use serenity::{model::channel::Message, Result};

pub fn check_msg(result: Result<Message>) {
    if let Err(why) = result {
        println!("Error sending message: {why:?}");
    }
}
