//! What the pointer does in the tree: which row it chose and which client it drags.

use crate::model::{ChannelId, ClientId};

/// A client being dragged onto a channel.
#[derive(Clone, Debug, PartialEq)]
pub struct Drag {
    pub client: ClientId,
    pub name: String,
    pub origin: (f32, f32),
    /// Whether the pointer moved far enough for the press to be a drag.
    pub active: bool,
}

/// What the tree has chosen.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Selection {
    None,
    Channel(ChannelId),
    Client(ClientId),
}
