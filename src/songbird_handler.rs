use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use serenity::async_trait;
use songbird::{Event, EventContext, EventHandler as VoiceEventHandler, Songbird};

use crate::WATCH_CHANNELS;

pub struct DriverDisconnectNotifier {
    pub songbird_manager: Arc<Songbird>,
}

#[async_trait]
impl VoiceEventHandler for DriverDisconnectNotifier {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        let EventContext::DriverDisconnect(ctx) = ctx else {
            return None;
        };

        if ctx.reason.is_some() {
            return None;
        }

        let manager = &self.songbird_manager;

        WATCH_CHANNELS
            .lock()
            .unwrap()
            .remove(&ctx.guild_id.0.into());

        manager
            .remove(ctx.guild_id)
            .await
            .expect("Failed to remove from manager");

        None
    }
}

pub struct ReadEndNotifier {
    pub temporary_filename: PathBuf,
}

#[async_trait]
impl VoiceEventHandler for ReadEndNotifier {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        if let EventContext::Track(_) = ctx {
            fs::remove_file(&self.temporary_filename).expect("Failed to remove temporary file");
        }

        None
    }
}
