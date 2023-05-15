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
