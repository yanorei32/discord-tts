use serenity::model::channel::Message;
use crate::SerenityResult;

pub trait LogSerenityError {
    fn log_error(&self);
}

impl LogSerenityError for SerenityResult<Message> {
    fn log_error(&self) {
        if let Err(why) = self {
            println!("Error sending message: {why:?}");
        }
    }
}