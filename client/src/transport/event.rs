//! What a transport tells the interface. Every transport reports the same events, so the interface
//! never learns which protocol carried them.

use std::collections::BTreeSet;

use crate::model::{Channel, ChatTarget, Client, ClientId, ServerInfo};
use crate::transport::Capabilities;

/// Where the client is in reaching a server.
#[derive(Clone, Debug, PartialEq)]
pub enum ConnStatus {
    Disconnected,
    Connecting(String),
    Connected,
    Failed(String),
}

/// Where the client is in carrying voice.
#[derive(Clone, Debug, PartialEq)]
pub enum MediaStatus {
    Off,
    Connecting,
    /// Carrying voice through `relay`, listening to `subscriptions` other clients.
    Connected {
        relay: String,
        subscriptions: usize,
    },
    Failed(String),
}

/// One thing that happened on the network.
#[derive(Clone, Debug)]
#[allow(dead_code, reason = "the bot prints events through Debug")]
pub enum Event {
    Status(ConnStatus),
    /// What the live transport can do. Sent once a session starts.
    Caps(Capabilities),
    Welcome {
        you: Client,
    },
    State {
        server: ServerInfo,
        channels: Vec<Channel>,
        clients: Vec<Client>,
    },
    Log {
        error: bool,
        text: String,
    },
    Chat {
        target: ChatTarget,
        from: ClientId,
        from_name: String,
        to: Option<ClientId>,
        text: String,
    },
    Poke {
        from: ClientId,
        from_name: String,
        text: String,
    },
    Talking(BTreeSet<ClientId>),
    Media(MediaStatus),
    /// Whether this client is sharing its screen right now.
    #[cfg(feature = "screen-share")]
    Sharing(bool),
}
