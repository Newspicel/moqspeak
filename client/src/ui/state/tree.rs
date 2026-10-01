//! The channel tree as a flat list of rows, in the order the tree shows them.

use std::collections::BTreeSet;

use crate::model::{Channel, ChannelId, Client, ClientId, Role};

/// What identifies a row across updates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RowKey {
    Channel(ChannelId),
    Client(ClientId),
}

/// One channel of the tree.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct ChannelRow {
    pub id: ChannelId,
    pub name: String,
    pub topic: String,
    pub depth: u16,
    pub full: bool,
    pub is_default: bool,
    /// Whether you are in it.
    pub mine: bool,
    pub count: usize,
    pub has_children: bool,
    pub collapsed: bool,
}

/// One client of the tree.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct ClientRow {
    pub id: ClientId,
    pub name: String,
    pub depth: u16,
    pub muted: bool,
    pub deaf: bool,
    pub away: bool,
    /// Whether this is you.
    pub me: bool,
    #[cfg(feature = "screen-share")]
    pub sharing: bool,
    pub role: Role,
}

/// One row of the tree.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Row {
    Channel(ChannelRow),
    Client(ClientRow),
}

impl Row {
    pub fn key(&self) -> RowKey {
        match self {
            Row::Channel(c) => RowKey::Channel(c.id),
            Row::Client(c) => RowKey::Client(c.id),
        }
    }
}

/// Flattens the channel tree and its clients in display order: each channel, then the clients
/// in it by name, then its sub-channels.
pub fn tree_rows(
    channels: &[Channel],
    clients: &[Client],
    me: Option<ClientId>,
    collapsed: &BTreeSet<ChannelId>,
) -> Vec<Row> {
    struct Walk<'a> {
        channels: &'a [Channel],
        clients: &'a [Client],
        me: Option<ClientId>,
        collapsed: &'a BTreeSet<ChannelId>,
    }
    fn walk(cx: &Walk<'_>, parent: Option<ChannelId>, depth: u16, rows: &mut Vec<Row>) {
        let mut children: Vec<&Channel> =
            cx.channels.iter().filter(|c| c.parent == parent).collect();
        children.sort_by_key(|c| (c.order, c.id));
        for ch in children {
            let mut inside: Vec<&Client> =
                cx.clients.iter().filter(|c| c.channel == ch.id).collect();
            inside.sort_by_key(|c| c.name.to_lowercase());
            let has_children =
                !inside.is_empty() || cx.channels.iter().any(|c| c.parent == Some(ch.id));
            let collapsed = cx.collapsed.contains(&ch.id);
            rows.push(Row::Channel(ChannelRow {
                id: ch.id,
                name: ch.name.clone(),
                topic: ch.topic.clone(),
                depth,
                full: ch.max_clients > 0 && inside.len() as u32 >= ch.max_clients,
                is_default: ch.is_default,
                mine: inside.iter().any(|c| Some(c.id) == cx.me),
                count: inside.len(),
                has_children,
                collapsed,
            }));
            if collapsed {
                continue;
            }
            for c in inside {
                rows.push(Row::Client(ClientRow {
                    id: c.id,
                    name: c.name.clone(),
                    depth: depth + 1,
                    muted: c.muted,
                    deaf: c.deaf,
                    away: c.away,
                    me: Some(c.id) == cx.me,
                    #[cfg(feature = "screen-share")]
                    sharing: c.sharing,
                    role: c.role,
                }));
            }
            walk(cx, Some(ch.id), depth + 1, rows);
        }
    }
    let mut rows = Vec::new();
    walk(
        &Walk {
            channels,
            clients,
            me,
            collapsed,
        },
        None,
        1,
        &mut rows,
    );
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ch(id: u64, name: &str, parent: Option<u64>, order: i64) -> Channel {
        Channel {
            id,
            name: name.into(),
            topic: String::new(),
            description: String::new(),
            parent,
            order,
            max_clients: 0,
            is_default: id == 1,
        }
    }

    fn cl(id: u64, name: &str, channel: u64) -> Client {
        Client {
            id,
            uid: String::new(),
            name: name.into(),
            channel,
            muted: false,
            deaf: false,
            away: false,
            away_message: String::new(),
            broadcast: String::new(),
            sharing: false,
            role: Default::default(),
            connected_at: 0,
            platform: String::new(),
            version: String::new(),
        }
    }

    fn names(rows: &[Row]) -> Vec<String> {
        rows.iter()
            .map(|r| match r {
                Row::Channel(c) => {
                    format!("{}#{}{}", c.depth, c.name, if c.mine { "*" } else { "" })
                }
                Row::Client(c) => format!("{}@{}{}", c.depth, c.name, if c.me { "*" } else { "" }),
            })
            .collect()
    }

    #[test]
    fn tree_is_depth_first_with_clients_before_subchannels() {
        let channels = vec![
            ch(1, "Lobby", None, 0),
            ch(2, "Gaming", None, 1),
            ch(3, "CS", Some(2), 0),
        ];
        let clients = vec![cl(10, "bob", 2), cl(11, "Alice", 2), cl(12, "carl", 3)];
        let rows = tree_rows(&channels, &clients, Some(11), &BTreeSet::new());
        assert_eq!(
            names(&rows),
            [
                "1#Lobby",
                "1#Gaming*",
                "2@Alice*",
                "2@bob",
                "2#CS",
                "3@carl"
            ]
        );
    }

    #[test]
    fn a_collapsed_channel_hides_its_clients_and_subchannels() {
        let channels = vec![ch(1, "Lobby", None, 0), ch(2, "Sub", Some(1), 0)];
        let clients = vec![cl(10, "bob", 1)];
        let collapsed = BTreeSet::from([1]);
        let rows = tree_rows(&channels, &clients, None, &collapsed);
        assert_eq!(names(&rows), ["1#Lobby"]);
    }
}
