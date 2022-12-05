use serde::{Deserialize, Serialize};
use serenity::model::prelude::UserId;
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub struct UserSettings {
    pub speaker: Option<u8>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct State {
    pub user_settings: HashMap<UserId, UserSettings>,
}

#[derive(Eq, PartialEq, Copy, Clone)]
pub enum SpeakerSelector {
    SpeakerOnly { speaker: usize },
    SpeakerAndStyle { speaker: usize, style: usize },
    None,
}

impl SpeakerSelector {
    pub fn speaker(&self) -> Option<usize> {
        match self {
            SpeakerSelector::SpeakerAndStyle { speaker, .. }
            | SpeakerSelector::SpeakerOnly { speaker } => Some(*speaker),
            SpeakerSelector::None => None,
        }
    }

    pub fn style(&self) -> Option<usize> {
        match self {
            SpeakerSelector::SpeakerAndStyle { style, .. } => Some(*style),
            _ => None,
        }
    }
}
