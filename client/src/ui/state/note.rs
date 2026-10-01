//! Announcements: short notes the window shows in its corner.

use crate::model::ClientId;

/// How grave a note is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    Info,
    Error,
}

/// One announcement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Note {
    pub level: Level,
    pub title: String,
    pub detail: Option<String>,
    /// The client a "Reply" action writes to.
    pub reply: Option<ClientId>,
}
