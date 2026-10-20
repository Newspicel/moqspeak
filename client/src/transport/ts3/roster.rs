//! The channels and clients of a TeamSpeak server. A server volunteers all of it when a session
//! opens — the channel list in as many parts as it takes, then every client in view — and keeps it
//! current with notifications. Nothing here asks the server for a list: the commands that would
//! are the ones a server query sends, and a client is refused them.

use std::collections::HashMap;

use ts3proto::unescape;

use crate::model::{Channel, ChannelId, Client, ClientId, Role, ServerInfo};
use crate::transport::ts3::spacer;

/// A client the server reports as a query connection, which the interface does not show.
const QUERY_CLIENT: i64 = 1;

/// What a notification did to the roster.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Change {
    /// Nothing the interface shows.
    Quiet,
    /// The lists changed and the interface should see them.
    State,
}

/// Everything the interface shows about one TeamSpeak server.
pub struct Roster {
    pub server: ServerInfo,
    pub channels: Vec<Channel>,
    pub clients: Vec<Client>,
    /// The channel rows as the server last described them, in the order it sent them.
    rows: Vec<HashMap<String, String>>,
    /// What the server calls each standing it grants, by its id.
    groups: HashMap<u64, String>,
    /// The standings each client holds, kept so that a group list arriving later still names them.
    held: HashMap<ClientId, Vec<u64>>,
}

impl Roster {
    pub fn new(relay: String) -> Self {
        Self {
            server: ServerInfo {
                name: String::new(),
                welcome: String::new(),
                created_at: 0,
                relay,
            },
            channels: Vec::new(),
            clients: Vec::new(),
            rows: Vec::new(),
            groups: HashMap::new(),
            held: HashMap::new(),
        }
    }

    /// Reads the channels out of the rows the server sent.
    fn rebuild(&mut self) {
        self.channels = channels_from(&self.rows);
    }

    /// Takes one channel row, replacing what was known of that channel field by field.
    fn remember_channel(&mut self, params: &HashMap<String, String>) {
        let Some(id) = number::<ChannelId>(params, "cid") else {
            return;
        };
        match self
            .rows
            .iter_mut()
            .find(|row| number::<ChannelId>(row, "cid") == Some(id))
        {
            Some(row) => row.extend(params.iter().map(|(k, v)| (k.clone(), v.clone()))),
            None => self.rows.push(params.clone()),
        }
    }

    /// Names the standing of every client from the groups the server granted it.
    fn rank_everyone(&mut self) {
        for client in &mut self.clients {
            client.rank = self
                .held
                .get(&client.id)
                .and_then(|ids| ids.iter().find_map(|id| self.groups.get(id)))
                .cloned()
                .unwrap_or_default();
        }
    }

    /// Keeps the standings a client row names, and gives the client its name for them.
    fn rank_one(&mut self, id: ClientId, params: &HashMap<String, String>) {
        if let Some(ids) = params.get("client_servergroups") {
            let ids: Vec<u64> = ids
                .split(',')
                .filter_map(|i| i.trim().parse().ok())
                .collect();
            self.held.insert(id, ids);
        }
        let rank = self
            .held
            .get(&id)
            .and_then(|ids| ids.iter().find_map(|i| self.groups.get(i)))
            .cloned()
            .unwrap_or_default();
        if let Some(client) = self.clients.iter_mut().find(|c| c.id == id) {
            client.rank = rank;
        }
    }

    /// The client with this id.
    pub fn client(&self, id: ClientId) -> Option<&Client> {
        self.clients.iter().find(|c| c.id == id)
    }

    /// Takes one command from the server into the roster.
    pub fn apply(&mut self, name: &str, params: &HashMap<String, String>) -> Change {
        match name {
            "initserver" => {
                self.server.name = text(params, "virtualserver_name");
                self.server.welcome = text(params, "virtualserver_welcomemessage");
                Change::State
            }
            "notifycliententerview" => {
                let Some(id) = number::<ClientId>(params, "clid") else {
                    return Change::Quiet;
                };
                // An entering client names its channel `ctid`, not `cid`.
                let channel = number::<ChannelId>(params, "ctid");
                match self.clients.iter_mut().find(|c| c.id == id) {
                    // Already in view. Keep what the notification leaves out.
                    Some(existing) => {
                        update(existing, params);
                        if let Some(channel) = channel {
                            existing.channel = channel;
                        }
                    }
                    None => {
                        let Some(mut entering) = client_from(params) else {
                            return Change::Quiet;
                        };
                        if let Some(channel) = channel {
                            entering.channel = channel;
                        }
                        self.clients.push(entering);
                    }
                }
                self.rank_one(id, params);
                Change::State
            }
            "notifyclientleftview" => {
                let Some(id) = number::<ClientId>(params, "clid") else {
                    return Change::Quiet;
                };
                let before = self.clients.len();
                self.clients.retain(|c| c.id != id);
                if self.clients.len() == before {
                    Change::Quiet
                } else {
                    Change::State
                }
            }
            "notifyclientmoved" => {
                let Some(id) = number::<ClientId>(params, "clid") else {
                    return Change::Quiet;
                };
                let Some(channel) = number::<ChannelId>(params, "ctid") else {
                    return Change::Quiet;
                };
                match self.clients.iter_mut().find(|c| c.id == id) {
                    Some(client) => {
                        client.channel = channel;
                        Change::State
                    }
                    None => Change::Quiet,
                }
            }
            "notifyclientupdated" => {
                let Some(id) = number::<ClientId>(params, "clid") else {
                    return Change::Quiet;
                };
                if !self.clients.iter().any(|c| c.id == id) {
                    return Change::Quiet;
                }
                if let Some(client) = self.clients.iter_mut().find(|c| c.id == id) {
                    update(client, params);
                }
                self.rank_one(id, params);
                Change::State
            }
            // The channel list arrives in parts and is only whole at the end of it.
            "channellist" => {
                self.remember_channel(params);
                Change::Quiet
            }
            "channellistfinished" => {
                self.rebuild();
                Change::State
            }
            "notifychannelcreated" | "notifychanneledited" | "notifychannelmoved" => {
                self.remember_channel(params);
                self.rebuild();
                Change::State
            }
            "notifychanneldeleted" => {
                let Some(id) = number::<ChannelId>(params, "cid") else {
                    return Change::Quiet;
                };
                self.rows
                    .retain(|row| number::<ChannelId>(row, "cid") != Some(id));
                self.rebuild();
                Change::State
            }
            "notifyservergrouplist" => {
                let Some(id) = number::<u64>(params, "sgid") else {
                    return Change::Quiet;
                };
                self.groups.insert(id, text(params, "name"));
                self.rank_everyone();
                Change::State
            }
            _ => Change::Quiet,
        }
    }
}

/// Reads the channel rows, in the order the server shows them.
fn channels_from(rows: &[HashMap<String, String>]) -> Vec<Channel> {
    let rows = newest(rows, "cid");
    let order = sibling_order(&rows);
    rows.iter()
        .filter_map(|row| {
            let id: ChannelId = number(row, "cid")?;
            let parent = parent_of(row).filter(|p| *p != 0);
            let (spacer, name) = spacer::split(&text(row, "channel_name"));
            Some(Channel {
                id,
                name,
                spacer,
                topic: text(row, "channel_topic"),
                description: String::new(),
                parent,
                order: order.get(&id).copied().unwrap_or(0),
                max_clients: number(row, "channel_maxclients").unwrap_or(0),
                is_default: number::<i64>(row, "channel_flag_default").unwrap_or(0) == 1,
            })
        })
        .collect()
}

/// The channel a row hangs under. A list the server pushes names it `cpid`.
fn parent_of(row: &HashMap<String, String>) -> Option<ChannelId> {
    number::<ChannelId>(row, "cpid").or_else(|| number::<ChannelId>(row, "pid"))
}

/// TeamSpeak orders channels by naming the one before each of them. This turns that chain into an
/// index per parent, which is what the tree sorts on.
fn sibling_order(rows: &[&HashMap<String, String>]) -> HashMap<ChannelId, i64> {
    let mut after: HashMap<ChannelId, HashMap<ChannelId, ChannelId>> = HashMap::new();
    let mut parents: HashMap<ChannelId, ChannelId> = HashMap::new();
    for row in rows {
        let Some(id) = number::<ChannelId>(row, "cid") else {
            continue;
        };
        let parent = parent_of(row).unwrap_or(0);
        let previous = number::<ChannelId>(row, "channel_order").unwrap_or(0);
        after.entry(parent).or_default().insert(previous, id);
        parents.insert(id, parent);
    }
    let mut order = HashMap::new();
    for (parent, chain) in &after {
        let mut index = 0;
        let mut previous = 0;
        // Follow the chain from its head, guarding against a cycle the server should not send.
        for _ in 0..chain.len() {
            let Some(&id) = chain.get(&previous) else {
                break;
            };
            order.insert(id, index);
            index += 1;
            previous = id;
        }
        // Anything the chain left out keeps the order the server listed it in.
        for row in rows {
            if let Some(id) = number::<ChannelId>(row, "cid")
                && parents.get(&id) == Some(parent)
                && !order.contains_key(&id)
            {
                order.insert(id, index);
                index += 1;
            }
        }
    }
    order
}

/// Keeps the last row for each id, in the order the ids first appeared. A server sends its
/// channel list when a session opens and answers a request for it as well, so the same channel
/// can arrive twice, and the later copy is the current one.
fn newest<'a>(rows: &'a [HashMap<String, String>], key: &str) -> Vec<&'a HashMap<String, String>> {
    let mut at: HashMap<u64, usize> = HashMap::new();
    let mut out: Vec<&HashMap<String, String>> = Vec::new();
    for row in rows {
        let Some(id) = number::<u64>(row, key) else {
            continue;
        };
        match at.get(&id) {
            Some(&index) => out[index] = row,
            None => {
                at.insert(id, out.len());
                out.push(row);
            }
        }
    }
    out
}

/// Reads one client row, or nothing where the row describes a query connection.
fn client_from(row: &HashMap<String, String>) -> Option<Client> {
    let id: ClientId = number(row, "clid")?;
    if number::<i64>(row, "client_type").unwrap_or(0) == QUERY_CLIENT {
        return None;
    }
    let mut client = Client {
        id,
        uid: text(row, "client_unique_identifier"),
        name: text(row, "client_nickname"),
        channel: number(row, "cid").unwrap_or(0),
        muted: false,
        deaf: false,
        away: false,
        away_message: String::new(),
        // TeamSpeak carries voice on the session itself, so there is no path to publish at.
        media: String::new(),
        sharing: false,
        // TeamSpeak grants server groups, which do not map onto these three roles. The group
        // the server names for this client becomes its rank instead.
        role: Role::User,
        rank: String::new(),
        connected_at: 0,
        platform: String::new(),
        version: String::new(),
    };
    update(&mut client, row);
    Some(client)
}

/// Takes the fields a row carries into a client, leaving the rest as it is.
fn update(client: &mut Client, row: &HashMap<String, String>) {
    if let Some(name) = row.get("client_nickname") {
        client.name = unescape(name);
    }
    if let Some(uid) = row.get("client_unique_identifier") {
        client.uid = uid.clone();
    }
    if let Some(muted) = number::<i64>(row, "client_input_muted") {
        client.muted = muted == 1;
    }
    if let Some(deaf) = number::<i64>(row, "client_output_muted") {
        client.deaf = deaf == 1;
    }
    if let Some(away) = number::<i64>(row, "client_away") {
        client.away = away == 1;
    }
    if let Some(message) = row.get("client_away_message") {
        client.away_message = unescape(message);
    }
    if let Some(channel) = number::<ChannelId>(row, "cid") {
        client.channel = channel;
    }
}

/// An unescaped text field, empty where the row does not carry it.
fn text(row: &HashMap<String, String>, key: &str) -> String {
    row.get(key).map(|v| unescape(v)).unwrap_or_default()
}

/// A numeric field, where the row carries one that parses.
fn number<T: std::str::FromStr>(row: &HashMap<String, String>, key: &str) -> Option<T> {
    row.get(key)?.trim().parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect()
    }

    #[test]
    fn reads_a_channel_list_in_the_servers_order() {
        let rows = vec![
            row(&[
                ("cid", "3"),
                ("pid", "0"),
                ("channel_order", "1"),
                ("channel_name", "Third"),
            ]),
            row(&[
                ("cid", "1"),
                ("pid", "0"),
                ("channel_order", "0"),
                ("channel_name", "First"),
            ]),
            row(&[
                ("cid", "2"),
                ("pid", "1"),
                ("channel_order", "0"),
                ("channel_name", "Child"),
            ]),
        ];
        let channels = channels_from(&rows);
        let first = channels.iter().find(|c| c.id == 1).unwrap();
        let third = channels.iter().find(|c| c.id == 3).unwrap();
        let child = channels.iter().find(|c| c.id == 2).unwrap();
        assert_eq!(first.order, 0);
        assert_eq!(third.order, 1);
        assert_eq!(first.parent, None);
        assert_eq!(child.parent, Some(1));
        assert_eq!(child.order, 0);
    }

    #[test]
    fn unescapes_the_names_teamspeak_escapes() {
        let rows = vec![row(&[
            ("cid", "1"),
            ("pid", "0"),
            ("channel_order", "0"),
            ("channel_name", "Lobby\\sand\\sFriends"),
        ])];
        assert_eq!(channels_from(&rows)[0].name, "Lobby and Friends");
    }

    #[test]
    fn a_channel_sent_twice_is_listed_once() {
        let rows = vec![
            row(&[
                ("cid", "1"),
                ("pid", "0"),
                ("channel_order", "0"),
                ("channel_name", "Old"),
            ]),
            row(&[
                ("cid", "2"),
                ("pid", "0"),
                ("channel_order", "1"),
                ("channel_name", "Second"),
            ]),
            // The server sent the list again, and the name had changed meanwhile.
            row(&[
                ("cid", "1"),
                ("pid", "0"),
                ("channel_order", "0"),
                ("channel_name", "New"),
            ]),
        ];
        let channels = channels_from(&rows);
        assert_eq!(channels.len(), 2);
        assert_eq!(channels[0].name, "New");
        assert_eq!(channels[0].order, 0);
        assert_eq!(channels[1].order, 1);
    }

    #[test]
    fn an_entering_client_already_in_view_keeps_what_it_had() {
        let mut roster = Roster::new("example:9987".into());
        roster.clients = vec![
            client_from(&row(&[
                ("clid", "7"),
                ("cid", "1"),
                ("client_nickname", "Ada"),
                ("client_input_muted", "1"),
            ]))
            .unwrap(),
        ];
        // The entry names the channel but not the mute flag.
        roster.apply(
            "notifycliententerview",
            &row(&[("clid", "7"), ("ctid", "5")]),
        );
        let client = roster.client(7).unwrap();
        assert_eq!(client.channel, 5);
        assert_eq!(client.name, "Ada");
        assert!(client.muted);
    }

    #[test]
    fn an_entry_without_a_channel_leaves_the_one_it_had() {
        let mut roster = Roster::new("example:9987".into());
        roster.clients = vec![client_from(&row(&[("clid", "7"), ("cid", "3")])).unwrap()];
        roster.apply("notifycliententerview", &row(&[("clid", "7")]));
        assert_eq!(roster.client(7).unwrap().channel, 3);
    }

    #[test]
    fn leaves_out_query_clients() {
        assert!(client_from(&row(&[("clid", "5"), ("client_type", "1")])).is_none());
        assert!(client_from(&row(&[("clid", "5"), ("client_type", "0")])).is_some());
    }

    #[test]
    fn reads_the_voice_flags() {
        let c = client_from(&row(&[
            ("clid", "7"),
            ("cid", "2"),
            ("client_nickname", "Ada"),
            ("client_input_muted", "1"),
            ("client_output_muted", "0"),
            ("client_away", "1"),
            ("client_away_message", "back\\ssoon"),
        ]))
        .unwrap();
        assert_eq!(c.name, "Ada");
        assert_eq!(c.channel, 2);
        assert!(c.muted);
        assert!(!c.deaf);
        assert!(c.away);
        assert_eq!(c.away_message, "back soon");
    }

    #[test]
    fn an_update_changes_only_what_it_carries() {
        let mut roster = Roster::new("example:9987".into());
        roster.clients = vec![
            client_from(&row(&[
                ("clid", "7"),
                ("client_nickname", "Ada"),
                ("cid", "1"),
            ]))
            .unwrap(),
        ];
        let change = roster.apply(
            "notifyclientupdated",
            &row(&[("clid", "7"), ("client_input_muted", "1")]),
        );
        assert_eq!(change, Change::State);
        let client = roster.client(7).unwrap();
        assert!(client.muted);
        // The nickname was not in the notification, so it stays.
        assert_eq!(client.name, "Ada");
        assert_eq!(client.channel, 1);
    }

    #[test]
    fn a_move_follows_the_target_channel() {
        let mut roster = Roster::new("example:9987".into());
        roster.clients = vec![client_from(&row(&[("clid", "7"), ("cid", "1")])).unwrap()];
        roster.apply("notifyclientmoved", &row(&[("clid", "7"), ("ctid", "4")]));
        assert_eq!(roster.client(7).unwrap().channel, 4);
    }

    #[test]
    fn an_entering_client_takes_its_channel_from_ctid() {
        let mut roster = Roster::new("example:9987".into());
        let change = roster.apply(
            "notifycliententerview",
            &row(&[("clid", "9"), ("ctid", "3"), ("client_nickname", "Grace")]),
        );
        assert_eq!(change, Change::State);
        assert_eq!(roster.client(9).unwrap().channel, 3);
        assert_eq!(roster.client(9).unwrap().name, "Grace");
    }

    #[test]
    fn a_leaving_client_goes() {
        let mut roster = Roster::new("example:9987".into());
        roster.clients = vec![client_from(&row(&[("clid", "7")])).unwrap()];
        assert_eq!(
            roster.apply("notifyclientleftview", &row(&[("clid", "7")])),
            Change::State
        );
        assert!(roster.client(7).is_none());
        // A second notification for the same client changes nothing.
        assert_eq!(
            roster.apply("notifyclientleftview", &row(&[("clid", "7")])),
            Change::Quiet
        );
    }

    #[test]
    fn takes_the_server_name_from_initserver() {
        let mut roster = Roster::new("example:9987".into());
        roster.apply(
            "initserver",
            &row(&[
                ("virtualserver_name", "Ada\\sand\\sfriends"),
                ("virtualserver_welcomemessage", "Be\\skind"),
            ]),
        );
        assert_eq!(roster.server.name, "Ada and friends");
        assert_eq!(roster.server.welcome, "Be kind");
        assert_eq!(roster.server.relay, "example:9987");
    }

    /// The server sends its channels in as many parts as it likes, and the list is only whole
    /// when it says so.
    #[test]
    fn the_channel_list_arrives_in_parts() {
        let mut roster = Roster::new("example:9987".into());
        for (cid, name) in [("1", "First"), ("2", "Second")] {
            let change = roster.apply(
                "channellist",
                &row(&[
                    ("cid", cid),
                    ("cpid", "0"),
                    ("channel_order", "0"),
                    ("channel_name", name),
                ]),
            );
            // Nothing to show until the list is whole.
            assert_eq!(change, Change::Quiet);
            assert!(roster.channels.is_empty());
        }
        assert_eq!(
            roster.apply("channellistfinished", &row(&[])),
            Change::State
        );
        assert_eq!(roster.channels.len(), 2);
    }

    #[test]
    fn a_pushed_row_names_its_parent_cpid() {
        let mut roster = Roster::new("example:9987".into());
        roster.apply(
            "channellist",
            &row(&[("cid", "1"), ("cpid", "0"), ("channel_name", "Top")]),
        );
        roster.apply(
            "channellist",
            &row(&[("cid", "2"), ("cpid", "1"), ("channel_name", "Under")]),
        );
        roster.apply("channellistfinished", &row(&[]));
        let under = roster.channels.iter().find(|c| c.id == 2).unwrap();
        assert_eq!(under.parent, Some(1));
    }

    #[test]
    fn an_edited_channel_keeps_the_fields_the_notice_leaves_out() {
        let mut roster = Roster::new("example:9987".into());
        roster.apply(
            "channellist",
            &row(&[("cid", "1"), ("cpid", "0"), ("channel_name", "Before")]),
        );
        roster.apply("channellistfinished", &row(&[]));
        roster.apply(
            "notifychanneledited",
            &row(&[("cid", "1"), ("channel_name", "After")]),
        );
        assert_eq!(roster.channels[0].name, "After");
        assert_eq!(roster.channels[0].parent, None);
    }

    #[test]
    fn a_deleted_channel_goes() {
        let mut roster = Roster::new("example:9987".into());
        roster.apply(
            "channellist",
            &row(&[("cid", "1"), ("channel_name", "Gone")]),
        );
        roster.apply("channellistfinished", &row(&[]));
        assert_eq!(roster.channels.len(), 1);
        roster.apply("notifychanneldeleted", &row(&[("cid", "1")]));
        assert!(roster.channels.is_empty());
    }

    #[test]
    fn a_client_wears_the_name_of_the_group_it_holds() {
        let mut roster = Roster::new("example:9987".into());
        roster.apply(
            "notifyservergrouplist",
            &row(&[("sgid", "9"), ("name", "Technik")]),
        );
        roster.apply(
            "notifycliententerview",
            &row(&[
                ("clid", "7"),
                ("ctid", "1"),
                ("client_nickname", "Ada"),
                ("client_servergroups", "9"),
            ]),
        );
        assert_eq!(roster.client(7).unwrap().rank, "Technik");
    }

    /// A server names its groups after it has named the clients in them.
    #[test]
    fn a_group_list_that_arrives_late_still_names_everyone() {
        let mut roster = Roster::new("example:9987".into());
        roster.apply(
            "notifycliententerview",
            &row(&[("clid", "7"), ("ctid", "1"), ("client_servergroups", "3,9")]),
        );
        assert_eq!(roster.client(7).unwrap().rank, "");
        roster.apply(
            "notifyservergrouplist",
            &row(&[("sgid", "3"), ("name", "Server\\sAdmin")]),
        );
        assert_eq!(roster.client(7).unwrap().rank, "Server Admin");
    }

    #[test]
    fn a_client_in_no_named_group_wears_nothing() {
        let mut roster = Roster::new("example:9987".into());
        roster.apply(
            "notifycliententerview",
            &row(&[("clid", "7"), ("ctid", "1"), ("client_servergroups", "404")]),
        );
        assert_eq!(roster.client(7).unwrap().rank, "");
    }
}
