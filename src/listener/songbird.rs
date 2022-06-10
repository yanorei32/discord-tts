use std::path::PathBuf;
use std::sync::Arc;
use songbird::{EventContext, Songbird};
use songbird::Event;
use songbird::events::EventHandler;
use serenity::async_trait;

pub struct ReadEndNotifier {
    pub temporary_filename: PathBuf,
}

#[async_trait]
impl EventHandler for ReadEndNotifier {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        if let EventContext::Track(_) = ctx {
            std::fs::remove_file(&self.temporary_filename).expect("Failed to remove temporary file")
        }
        None
    }
}

pub struct DriverDisconnectNotifier {
    pub songbird_manager: Arc<Songbird>,
}

#[async_trait]
impl EventHandler for DriverDisconnectNotifier {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        if let EventContext::DriverDisconnect(ctx) = ctx {
            let guild_id = ctx.guild_id;
            let manager = &self.songbird_manager;
            let has_handler = manager.get(guild_id).is_some();

            println!("Force disconnected");

            if has_handler {
                manager
                    .remove(guild_id)
                    .await
                    .expect("Failed to remove from manager");
            }
        }
        None
    }
}