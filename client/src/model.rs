//! The control protocol spoken with the moqspeak Worker.

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
    pub broadcast: String,
    #[serde(default)]
    pub sharing: bool,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChatTarget {
    Server,
    Channel,
    Client,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "t", rename_all = "snake_case")]
#[allow(
    dead_code,
    reason = "the protocol carries fields this client does not show yet"
)]
pub enum ServerMsg {
    Welcome {
        you: Client,
        relay: String,
        #[serde(default)]
        token: Option<String>,
    },
    State {
        server: ServerInfo,
        channels: Vec<Channel>,
        clients: Vec<Client>,
    },
    Event {
        kind: String,
        text: String,
    },
    Chat {
        target: ChatTarget,
        from: ClientId,
        from_name: String,
        #[serde(default)]
        to: Option<ClientId>,
        text: String,
        #[serde(default)]
        at: u64,
    },
    Poke {
        from: ClientId,
        from_name: String,
        text: String,
    },
    Error {
        message: String,
    },
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "t", rename_all = "snake_case")]
#[allow(
    dead_code,
    reason = "the server accepts commands this client has no control for yet"
)]
pub enum ClientMsg {
    Hello {
        name: String,
        uid: String,
        platform: String,
        version: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        channel: Option<ChannelId>,
    },
    Join {
        channel: ChannelId,
    },
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
    Move {
        id: ClientId,
        channel: ChannelId,
    },
    Sharing {
        sharing: bool,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_state() {
        let raw = r#"{"t":"state","server":{"name":"demo","welcome":"hi","created_at":1,"relay":"r"},
            "channels":[{"id":1,"name":"Lobby","topic":"","description":"","parent":null,"order":0,
            "max_clients":0,"is_default":true,"permanent":true}],
            "clients":[{"id":3,"uid":"u","name":"A","channel":1,"muted":false,"deaf":false,
            "away":false,"away_message":"","broadcast":"moqspeak/demo/3-x","connected_at":5,
            "platform":"macOS","version":"0.1"}]}"#;
        let ServerMsg::State {
            channels, clients, ..
        } = serde_json::from_str(raw).unwrap()
        else {
            panic!()
        };
        assert_eq!(channels[0].name, "Lobby");
        assert_eq!(clients[0].broadcast, "moqspeak/demo/3-x");
    }

    #[test]
    fn writes_hello() {
        let m = ClientMsg::Hello {
            name: "A".into(),
            uid: "u".into(),
            platform: "macOS".into(),
            version: "0.1".into(),
            channel: None,
        };
        assert_eq!(
            serde_json::to_string(&m).unwrap(),
            r#"{"t":"hello","name":"A","uid":"u","platform":"macOS","version":"0.1"}"#
        );
    }
}
