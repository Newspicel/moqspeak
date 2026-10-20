//! The things a voice server is made of: channels, the clients in them, and who may do what.
//! Every transport reports the same shapes, whatever its protocol calls them.

use serde::{Deserialize, Serialize};

pub type ChannelId = u64;
pub type ClientId = u64;

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Channel {
    pub id: ChannelId,
    pub name: String,
    #[serde(default)]
    pub topic: String,
    #[serde(default)]
    pub description: String,
    pub parent: Option<ChannelId>,
    #[serde(default)]
    pub order: i64,
    #[serde(default)]
    pub max_clients: u32,
    #[serde(default)]
    pub is_default: bool,
    /// Set where the channel rules a line or sets a heading instead of naming a place to talk.
    #[serde(default)]
    pub spacer: Option<Spacer>,
}

/// How a channel's name sits in the tree. TeamSpeak servers group their tree with these.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Spacer {
    /// The label repeats across the width, which rules a line.
    Fill,
    Left,
    Center,
    Right,
}

impl Spacer {
    pub fn name(self) -> &'static str {
        match self {
            Spacer::Fill => "fill",
            Spacer::Left => "left",
            Spacer::Center => "center",
            Spacer::Right => "right",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Client {
    pub id: ClientId,
    #[serde(default)]
    pub uid: String,
    pub name: String,
    pub channel: ChannelId,
    #[serde(default)]
    pub muted: bool,
    #[serde(default)]
    pub deaf: bool,
    #[serde(default)]
    pub away: bool,
    #[serde(default)]
    pub away_message: String,
    /// Where this client's voice and screen live. A path for a transport that carries media on
    /// its own connections, and empty for one that carries them on the session itself.
    #[serde(rename = "broadcast")]
    pub media: String,
    #[serde(default)]
    pub sharing: bool,
    #[serde(default)]
    pub role: Role,
    #[serde(default)]
    pub connected_at: u64,
    #[serde(default)]
    pub platform: String,
    #[serde(default)]
    pub version: String,
}

#[derive(Clone, Debug, PartialEq, Default, Deserialize)]
pub struct ServerInfo {
    pub name: String,
    #[serde(default)]
    pub welcome: String,
    #[serde(default)]
    pub created_at: u64,
    #[serde(default)]
    pub relay: String,
}

/// What a client may do on a server.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    #[default]
    User,
    Mod,
    Admin,
}

impl Role {
    pub fn label(self) -> &'static str {
        match self {
            Role::User => "User",
            Role::Mod => "Mod",
            Role::Admin => "Admin",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChatTarget {
    Server,
    Channel,
    Client,
}
