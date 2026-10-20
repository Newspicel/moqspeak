//! What the interface asks a server to do. Every transport reads the same actions and speaks them
//! in its own protocol. The moqspeak Worker takes them as its wire format directly.

use serde::Serialize;

use crate::model::{ChannelId, ChatTarget, ClientId, Role};

/// One request from the interface to the server.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "t", rename_all = "snake_case")]
#[allow(
    dead_code,
    reason = "a transport accepts actions this client has no control for yet"
)]
pub enum Action {
    /// Moves yourself into a channel.
    Join {
        channel: ChannelId,
    },
    /// Sets any of the flags that describe you. A field left out stays as it is.
    Status {
        #[serde(skip_serializing_if = "Option::is_none")]
        muted: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        deaf: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        away: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        away_message: Option<String>,
    },
    Rename {
        name: String,
    },
    Chat {
        target: ChatTarget,
        #[serde(skip_serializing_if = "Option::is_none")]
        to: Option<ClientId>,
        text: String,
    },
    Poke {
        to: ClientId,
        text: String,
    },
    CreateChannel {
        name: String,
        topic: String,
        description: String,
        parent: Option<ChannelId>,
        max_clients: u32,
    },
    EditChannel {
        id: ChannelId,
        name: String,
        topic: String,
        description: String,
        max_clients: u32,
    },
    DeleteChannel {
        id: ChannelId,
    },
    Kick {
        id: ClientId,
        reason: String,
    },
    /// Moves somebody else into a channel.
    Move {
        id: ClientId,
        channel: ChannelId,
    },
    Sharing {
        sharing: bool,
    },
    SetRole {
        id: ClientId,
        role: Role,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_the_worker_wire_format() {
        let join = Action::Join { channel: 4 };
        assert_eq!(
            serde_json::to_string(&join).unwrap(),
            r#"{"t":"join","channel":4}"#
        );
        let status = Action::Status {
            muted: Some(true),
            deaf: None,
            away: None,
            away_message: None,
        };
        assert_eq!(
            serde_json::to_string(&status).unwrap(),
            r#"{"t":"status","muted":true}"#
        );
    }
}
