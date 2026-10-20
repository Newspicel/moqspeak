//! The JSON control protocol the moqspeak Worker speaks. The Worker takes [`Action`] as it
//! stands, so only the handshake and the messages the server sends need a type of their own.

use serde::{Deserialize, Serialize};

use crate::model::{Channel, ChannelId, ChatTarget, Client, ClientId, ServerInfo};

/// What the Worker sends.
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "t", rename_all = "snake_case")]
#[allow(
    dead_code,
    reason = "the protocol carries fields this client does not show yet"
)]
pub enum ServerMsg {
    Challenge {
        nonce: String,
        server: String,
    },
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

/// What the client sends to open a session and prove who it is.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "t", rename_all = "snake_case")]
pub enum Handshake {
    Hello {
        name: String,
        uid: String,
        platform: String,
        version: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        channel: Option<ChannelId>,
    },
    Auth {
        sig: String,
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
        assert_eq!(clients[0].media, "moqspeak/demo/3-x");
    }

    #[test]
    fn writes_hello() {
        let m = Handshake::Hello {
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
