//! The interface's state: everything the network reported, plus what the user chose.

use std::collections::{BTreeSet, HashMap};
use std::sync::atomic::Ordering;

use serde::{Deserialize, Serialize};
use zgui::prelude::*;
use zgui::reactive::StoredValue;

use crate::audio::VoiceMode;
use crate::engine::{Command, ConnStatus, Engine, Event, MediaStatus};
use crate::model::{Channel, ChannelId, ChatTarget, Client, ClientId, ClientMsg, ServerInfo};

/// A chat tab.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Tab {
    Server,
    Channel,
    Private(ClientId),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LineKind {
    Info,
    Error,
    Event,
    Chat { from: String, own: bool },
    Poke { from: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    pub id: u64,
    pub time: String,
    pub tab: Tab,
    pub kind: LineKind,
    pub text: String,
}

/// A client being dragged onto a channel.
#[derive(Clone, Debug, PartialEq)]
pub struct Drag {
    pub client: ClientId,
    pub name: String,
    pub origin: (f32, f32),
    pub active: bool,
}

/// What the tree has selected.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Selection {
    Server,
    Channel(ChannelId),
    Client(ClientId),
}

/// A saved server.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Bookmark {
    pub label: String,
    pub address: String,
    pub nickname: String,
}

/// What survives a restart.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub nickname: String,
    pub address: String,
    pub bookmarks: Vec<Bookmark>,
    pub voice_mode: String,
    pub threshold_db: f32,
    pub input_gain: f32,
    pub output_volume: f32,
    pub input_device: Option<String>,
    pub output_device: Option<String>,
    /// "system", "dark" or "light".
    pub theme_mode: String,
    pub noise_suppression: bool,
    pub smart_vad: bool,
    pub echo_cancellation: bool,
}

pub const DEFAULT_ADDRESS: &str = "moqspeak.newspicel.workers.dev/public";

impl Default for Settings {
    fn default() -> Self {
        let user = std::env::var("USER").unwrap_or_else(|_| "Guest".into());
        let mut nickname: String = user.chars().take(1).flat_map(char::to_uppercase).collect();
        nickname.extend(user.chars().skip(1));
        Self {
            nickname,
            address: DEFAULT_ADDRESS.into(),
            bookmarks: vec![Bookmark {
                label: "moqspeak Public".into(),
                address: DEFAULT_ADDRESS.into(),
                nickname: String::new(),
            }],
            voice_mode: "activation".into(),
            threshold_db: -50.0,
            input_gain: 1.0,
            output_volume: 1.0,
            input_device: None,
            output_device: None,
            theme_mode: "system".into(),
            noise_suppression: true,
            smart_vad: true,
            echo_cancellation: true,
        }
    }
}

impl Settings {
    fn path() -> Option<std::path::PathBuf> {
        directories::ProjectDirs::from("dev", "moqspeak", "moqspeak")
            .map(|d| d.config_dir().join("settings.json"))
    }

    pub fn load() -> Self {
        Self::path()
            .and_then(|p| std::fs::read(p).ok())
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let Some(path) = Self::path() else { return };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(json) = serde_json::to_vec_pretty(self) {
            let _ = std::fs::write(path, json);
        }
    }
}

pub fn mode_from_str(s: &str) -> VoiceMode {
    match s {
        "ptt" => VoiceMode::PushToTalk,
        "continuous" => VoiceMode::Continuous,
        _ => VoiceMode::Activation,
    }
}

pub fn mode_to_str(m: VoiceMode) -> &'static str {
    match m {
        VoiceMode::Activation => "activation",
        VoiceMode::PushToTalk => "ptt",
        VoiceMode::Continuous => "continuous",
    }
}

/// The colour scheme.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Theme {
    System,
    Dark,
    Light,
}

impl Theme {
    pub fn parse(s: &str) -> Self {
        match s {
            "light" => Theme::Light,
            "dark" => Theme::Dark,
            _ => Theme::System,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Theme::System => "system",
            Theme::Dark => "dark",
            Theme::Light => "light",
        }
    }
}

/// Which dialog is open. One at a time, as in the original.
#[derive(Clone, Debug, PartialEq)]
pub enum Modal {
    None,
    Connect,
    CreateChannel {
        parent: Option<ChannelId>,
    },
    Options,
    Poke {
        to: ClientId,
        name: String,
    },
    Poked {
        from: String,
        text: String,
    },
    Share {
        monitors: Vec<crate::screen::MonitorInfo>,
    },
    About,
}

#[derive(Clone, Copy)]
pub struct AppState {
    pub engine: StoredValue<Engine>,
    pub status: RwSignal<ConnStatus>,
    pub media: RwSignal<MediaStatus>,
    pub server: RwSignal<ServerInfo>,
    pub channels: RwSignal<Vec<Channel>>,
    pub clients: RwSignal<Vec<Client>>,
    pub me: RwSignal<Option<ClientId>>,
    pub talking: RwSignal<BTreeSet<ClientId>>,
    pub lines: RwSignal<Vec<Line>>,
    pub tabs: RwSignal<Vec<(ClientId, String)>>,
    pub tab: RwSignal<Tab>,
    pub unread: RwSignal<BTreeSet<String>>,
    pub selected: RwSignal<Selection>,
    pub settings: RwSignal<Settings>,
    pub modal: RwSignal<Modal>,
    pub mic_muted: RwSignal<bool>,
    pub deafened: RwSignal<bool>,
    pub away: RwSignal<bool>,
    pub voice_mode: RwSignal<VoiceMode>,
    pub theme: RwSignal<Theme>,
    pub sharing: RwSignal<bool>,
    pub drag: RwSignal<Option<Drag>>,
    pub drop_target: RwSignal<Option<ChannelId>>,
    pub pointer: RwSignal<(f32, f32)>,
    pub collapsed: RwSignal<BTreeSet<ChannelId>>,
    last_click: StoredValue<Option<(Selection, std::time::Instant)>>,
    /// The application root's owner. Windows opened from deep inside the tree are opened under
    /// it, because a window's handle state belongs to the owner `open` runs in (zortax/zgui#5).
    pub root: StoredValue<zgui::reactive::Owner>,
    pub ptt: RwSignal<bool>,
    pub local_mutes: RwSignal<BTreeSet<ClientId>>,
    pub volumes: RwSignal<HashMap<ClientId, f32>>,
    pub connected_at: RwSignal<Option<std::time::Instant>>,
    next_line: RwSignal<u64>,
}

fn now_hms() -> String {
    chrono::Local::now().format("%H:%M:%S").to_string()
}

impl AppState {
    pub fn new(engine: Engine) -> Self {
        let settings = Settings::load();
        let mode = mode_from_str(&settings.voice_mode);
        let theme = Theme::parse(&settings.theme_mode);
        let audio = &engine.audio.shared;
        audio.set_mode(mode);
        audio.threshold_db.set(settings.threshold_db);
        audio.input_gain.set(settings.input_gain);
        audio.master_volume.set(settings.output_volume);
        audio
            .noise_suppression
            .store(settings.noise_suppression, Ordering::Relaxed);
        audio.smart_vad.store(settings.smart_vad, Ordering::Relaxed);
        audio
            .echo_cancellation
            .store(settings.echo_cancellation, Ordering::Relaxed);
        Self {
            engine: StoredValue::new(engine),
            status: RwSignal::new(ConnStatus::Disconnected),
            media: RwSignal::new(MediaStatus::Off),
            server: RwSignal::new(ServerInfo::default()),
            channels: RwSignal::new(Vec::new()),
            clients: RwSignal::new(Vec::new()),
            me: RwSignal::new(None),
            talking: RwSignal::new(BTreeSet::new()),
            lines: RwSignal::new(Vec::new()),
            tabs: RwSignal::new(Vec::new()),
            tab: RwSignal::new(Tab::Server),
            unread: RwSignal::new(BTreeSet::new()),
            selected: RwSignal::new(Selection::Server),
            settings: RwSignal::new(settings),
            modal: RwSignal::new(Modal::None),
            mic_muted: RwSignal::new(false),
            deafened: RwSignal::new(false),
            away: RwSignal::new(false),
            voice_mode: RwSignal::new(mode),
            theme: RwSignal::new(theme),
            sharing: RwSignal::new(false),
            drag: RwSignal::new(None),
            drop_target: RwSignal::new(None),
            pointer: RwSignal::new((0.0, 0.0)),
            collapsed: RwSignal::new(BTreeSet::new()),
            last_click: StoredValue::new(None),
            root: StoredValue::new(
                zgui::reactive::Owner::current()
                    .expect("AppState is created inside the app's root owner"),
            ),
            ptt: RwSignal::new(false),
            local_mutes: RwSignal::new(BTreeSet::new()),
            volumes: RwSignal::new(HashMap::new()),
            connected_at: RwSignal::new(None),
            next_line: RwSignal::new(1),
        }
    }

    pub fn expect() -> Self {
        use_local_context::<Self>().expect("AppState is provided at the root")
    }

    pub fn send(&self, command: Command) {
        // During shutdown the state can be gone before a window's cleanup runs.
        let _ = self.engine.try_with_value(|e| e.send(command));
    }

    pub fn msg(&self, msg: ClientMsg) {
        self.send(Command::Send(msg));
    }

    pub fn connected(&self) -> bool {
        self.status.with(|s| *s == ConnStatus::Connected)
    }

    pub fn my_client(&self) -> Option<Client> {
        let me = self.me.get()?;
        self.clients
            .with(|c| c.iter().find(|c| c.id == me).cloned())
    }

    /// What this client may do on the server it is connected to.
    pub fn my_role(&self) -> crate::model::Role {
        self.my_client().map(|c| c.role).unwrap_or_default()
    }

    pub fn my_role_untracked(&self) -> crate::model::Role {
        let me = self.me.get_untracked();
        self.clients
            .with_untracked(|c| c.iter().find(|c| Some(c.id) == me).map(|c| c.role))
            .unwrap_or_default()
    }

    pub fn client(&self, id: ClientId) -> Option<Client> {
        self.clients
            .with(|c| c.iter().find(|c| c.id == id).cloned())
    }

    pub fn channel(&self, id: ChannelId) -> Option<Channel> {
        self.channels
            .with(|c| c.iter().find(|c| c.id == id).cloned())
    }

    pub fn push_line(&self, tab: Tab, kind: LineKind, text: impl Into<String>) {
        let id = self.next_line.get_untracked();
        self.next_line.set(id + 1);
        let key = tab_key(&tab);
        if self.tab.get_untracked() != tab {
            self.unread.update(|u| {
                u.insert(key);
            });
        }
        self.lines.update(|lines| {
            lines.push(Line {
                id,
                time: now_hms(),
                tab,
                kind,
                text: text.into(),
            });
            if lines.len() > 2000 {
                lines.drain(..500);
            }
        });
    }

    pub fn info(&self, text: impl Into<String>) {
        self.push_line(Tab::Server, LineKind::Info, text);
    }

    pub fn open_private(&self, id: ClientId) {
        let Some(client) = self.client(id) else {
            return;
        };
        if Some(id) == self.me.get_untracked() {
            return;
        }
        self.tabs.update(|tabs| {
            if !tabs.iter().any(|(t, _)| *t == id) {
                tabs.push((id, client.name.clone()));
            }
        });
        self.select_tab(Tab::Private(id));
    }

    pub fn select_tab(&self, tab: Tab) {
        let key = tab_key(&tab);
        self.unread.update(|u| {
            u.remove(&key);
        });
        self.tab.set(tab);
    }

    pub fn close_tab(&self, id: ClientId) {
        self.tabs.update(|tabs| tabs.retain(|(t, _)| *t != id));
        if self.tab.get_untracked() == Tab::Private(id) {
            self.tab.set(Tab::Server);
        }
    }

    pub fn update_settings(&self, f: impl FnOnce(&mut Settings)) {
        self.settings.update(f);
        self.settings.with_untracked(Settings::save);
    }

    pub fn connect(&self, address: String, nickname: String) {
        self.update_settings(|s| {
            s.address = address.clone();
            s.nickname = nickname.clone();
        });
        self.send(Command::Connect { address, nickname });
    }

    pub fn disconnect(&self) {
        self.send(Command::Disconnect);
    }

    pub fn join(&self, channel: ChannelId) {
        if self.my_client().is_some_and(|c| c.channel == channel) {
            return;
        }
        self.msg(ClientMsg::Join { channel });
    }

    pub fn set_mic_muted(&self, muted: bool) {
        self.mic_muted.set(muted);
        self.engine
            .with_value(|e| e.audio.shared.mic_muted.store(muted, Ordering::Relaxed));
        self.msg(ClientMsg::Status {
            muted: Some(muted),
            deaf: None,
            away: None,
            away_message: None,
        });
    }

    pub fn set_deafened(&self, deaf: bool) {
        self.deafened.set(deaf);
        self.engine
            .with_value(|e| e.audio.shared.deafened.store(deaf, Ordering::Relaxed));
        self.msg(ClientMsg::Status {
            muted: None,
            deaf: Some(deaf),
            away: None,
            away_message: None,
        });
    }

    pub fn set_away(&self, away: bool) {
        self.away.set(away);
        self.msg(ClientMsg::Status {
            muted: None,
            deaf: None,
            away: Some(away),
            away_message: Some(if away { "Away".into() } else { String::new() }),
        });
    }

    pub fn set_ptt(&self, down: bool) {
        if self.ptt.get_untracked() != down {
            self.ptt.set(down);
            self.engine
                .with_value(|e| e.audio.shared.ptt_down.store(down, Ordering::Relaxed));
        }
    }

    /// Records a click on `target` and says whether it completes a double click.
    ///
    /// zgui does not dispatch `double_click` yet (zortax/zgui#3), so the tree times its own.
    pub fn click(&self, target: Selection) -> bool {
        let now = std::time::Instant::now();
        let double = self.last_click.with_value(|last| {
            last.is_some_and(|(t, at)| t == target && now.duration_since(at).as_millis() < 400)
        });
        self.last_click
            .set_value(if double { None } else { Some((target, now)) });
        self.selected.set(target);
        double
    }

    /// Starts or stops sharing. With several monitors, asks which one first.
    pub fn toggle_share(&self) {
        if self.sharing.get_untracked() {
            self.send(Command::StopShare);
            return;
        }
        #[cfg(target_os = "macos")]
        tracing::debug!(
            "share requested; system picker available: {}",
            crate::screen::mac::picker_available()
        );
        #[cfg(target_os = "macos")]
        if crate::screen::mac::picker_available() {
            // Apple's picker chooses a window, an app or a display. Its answer arrives on another
            // thread and comes back to this one through the UI handle.
            let ui = ui();
            let state = *self;
            crate::screen::mac::pick(move |outcome| {
                ui.post(move || match outcome {
                    Ok(Some(picked)) => state.send(Command::StartSharePicked(
                        crate::screen::Handoff::new(picked),
                    )),
                    Ok(None) => {}
                    Err(e) => state.push_line(Tab::Server, LineKind::Error, format!("{e:#}")),
                });
            });
            return;
        }
        let monitors = crate::screen::list_monitors();
        match monitors.len() {
            0 => self.push_line(
                Tab::Server,
                LineKind::Error,
                "No screen to share. On macOS, allow moqspeak under System Settings → Privacy & Security → Screen Recording.",
            ),
            1 => self.send(Command::StartShare { monitor: monitors[0].id }),
            _ => self.modal.set(Modal::Share { monitors }),
        }
    }

    /// Moves a client: yourself through a join, anyone else through a move request.
    pub fn move_client(&self, client: ClientId, channel: ChannelId) {
        if Some(client) == self.me.get_untracked() {
            self.join(channel);
        } else if self.client(client).is_some_and(|c| c.channel != channel) {
            self.msg(ClientMsg::Move {
                id: client,
                channel,
            });
        }
    }

    /// Ends a drag, dropping onto the current target if there is one.
    pub fn finish_drag(&self) {
        if let Some(drag) = self.drag.get_untracked() {
            if drag.active {
                if let Some(target) = self.drop_target.get_untracked() {
                    self.move_client(drag.client, target);
                }
            }
            self.drag.set(None);
        }
        if self.drop_target.get_untracked().is_some() {
            self.drop_target.set(None);
        }
    }

    pub fn toggle_collapsed(&self, channel: ChannelId) {
        self.collapsed.update(|c| {
            if !c.remove(&channel) {
                c.insert(channel);
            }
        });
    }

    pub fn set_theme(&self, theme: Theme) {
        self.theme.set(theme);
        self.update_settings(|s| s.theme_mode = theme.name().into());
    }

    pub fn set_voice_mode(&self, mode: VoiceMode) {
        self.voice_mode.set(mode);
        self.engine.with_value(|e| e.audio.shared.set_mode(mode));
        self.update_settings(|s| s.voice_mode = mode_to_str(mode).into());
    }

    pub fn set_local_mute(&self, id: ClientId, muted: bool) {
        self.local_mutes.update(|m| {
            if muted {
                m.insert(id);
            } else {
                m.remove(&id);
            }
        });
        self.engine
            .with_value(|e| e.audio.mixer.set_muted(id, muted));
    }

    pub fn set_volume(&self, id: ClientId, volume: f32) {
        self.volumes.update(|v| {
            v.insert(id, volume);
        });
        self.engine
            .with_value(|e| e.audio.mixer.set_volume(id, volume));
    }

    pub fn send_chat(&self, text: String) {
        let text = text.trim().to_owned();
        if text.is_empty() {
            return;
        }
        let (target, to) = match self.tab.get_untracked() {
            Tab::Server => (ChatTarget::Server, None),
            Tab::Channel => (ChatTarget::Channel, None),
            Tab::Private(id) => (ChatTarget::Client, Some(id)),
        };
        self.msg(ClientMsg::Chat { target, to, text });
    }

    /// Applies one event from the network.
    pub fn apply(&self, event: Event) {
        match event {
            Event::Status(status) => {
                match &status {
                    ConnStatus::Connected => self.connected_at.set(Some(std::time::Instant::now())),
                    ConnStatus::Disconnected | ConnStatus::Failed(_) => {
                        self.connected_at.set(None);
                        self.clients.set(Vec::new());
                        self.channels.set(Vec::new());
                        self.me.set(None);
                        self.talking.set(BTreeSet::new());
                        self.selected.set(Selection::Server);
                    }
                    ConnStatus::Connecting(_) => {}
                }
                self.status.set(status);
            }
            Event::Welcome { you } => {
                self.me.set(Some(you.id));
                self.info(format!("Connected to server as \"{}\"", you.name));
                // Re-assert the local state the server does not remember.
                let (muted, deaf, away) = (
                    self.mic_muted.get_untracked(),
                    self.deafened.get_untracked(),
                    self.away.get_untracked(),
                );
                if muted || deaf || away {
                    self.msg(ClientMsg::Status {
                        muted: Some(muted),
                        deaf: Some(deaf),
                        away: Some(away),
                        away_message: None,
                    });
                }
            }
            Event::State {
                server,
                channels,
                clients,
            } => {
                let first = self.channels.with_untracked(Vec::is_empty);
                if first && !server.welcome.is_empty() {
                    self.push_line(Tab::Server, LineKind::Event, server.welcome.clone());
                }
                // Keep private tab names fresh.
                self.tabs.update(|tabs| {
                    for (id, name) in tabs.iter_mut() {
                        if let Some(c) = clients.iter().find(|c| c.id == *id) {
                            *name = c.name.clone();
                        }
                    }
                });
                if let Selection::Client(id) = self.selected.get_untracked() {
                    if !clients.iter().any(|c| c.id == id) {
                        self.selected.set(Selection::Server);
                    }
                }
                if let Selection::Channel(id) = self.selected.get_untracked() {
                    if !channels.iter().any(|c| c.id == id) {
                        self.selected.set(Selection::Server);
                    }
                }
                // A development aid for screenshots: MOQSPEAK_SELECT=<nickname>.
                if let Ok(wanted) = std::env::var("MOQSPEAK_SELECT") {
                    if self.selected.get_untracked() == Selection::Server {
                        if let Some(c) = clients.iter().find(|c| c.name == wanted) {
                            self.selected.set(Selection::Client(c.id));
                        }
                    }
                }
                self.server.set(server);
                self.channels.set(channels);
                self.clients.set(clients);
            }
            Event::Log { error, text } => {
                let kind = if error {
                    LineKind::Error
                } else {
                    LineKind::Event
                };
                self.push_line(Tab::Server, kind, text);
            }
            Event::Chat {
                target,
                from,
                from_name,
                to,
                text,
            } => {
                let me = self.me.get_untracked();
                let own = Some(from) == me;
                let tab = match target {
                    ChatTarget::Server => Tab::Server,
                    ChatTarget::Channel => Tab::Channel,
                    ChatTarget::Client => {
                        let peer = if own { to.unwrap_or(from) } else { from };
                        let name = self
                            .client(peer)
                            .map(|c| c.name)
                            .unwrap_or(from_name.clone());
                        self.tabs.update(|tabs| {
                            if !tabs.iter().any(|(t, _)| *t == peer) {
                                tabs.push((peer, name));
                            }
                        });
                        Tab::Private(peer)
                    }
                };
                self.push_line(
                    tab,
                    LineKind::Chat {
                        from: from_name,
                        own,
                    },
                    text,
                );
            }
            Event::Poke {
                from_name, text, ..
            } => {
                self.push_line(
                    Tab::Server,
                    LineKind::Poke {
                        from: from_name.clone(),
                    },
                    text.clone(),
                );
                self.modal.set(Modal::Poked {
                    from: from_name,
                    text,
                });
            }
            Event::Talking(set) => self.talking.set(set),
            Event::Sharing(on) => self.sharing.set(on),
            Event::Media(status) => {
                match &status {
                    MediaStatus::Connected { relay, .. }
                        if !matches!(self.media.get_untracked(), MediaStatus::Connected { .. }) =>
                    {
                        let _ = relay;
                        self.info("Voice connected");
                    }
                    _ => {}
                }
                self.media.set(status);
            }
        }
    }
}

pub fn tab_key(tab: &Tab) -> String {
    match tab {
        Tab::Server => "server".into(),
        Tab::Channel => "channel".into(),
        Tab::Private(id) => format!("p{id}"),
    }
}

/// One row of the server tree.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Row {
    Server {
        name: String,
    },
    Channel {
        id: ChannelId,
        name: String,
        depth: u16,
        full: bool,
        is_default: bool,
        spacer: bool,
        count: usize,
        has_children: bool,
        collapsed: bool,
    },
    Client {
        id: ClientId,
        name: String,
        depth: u16,
        muted: bool,
        deaf: bool,
        away: bool,
        me: bool,
        sharing: bool,
        role: crate::model::Role,
    },
}

/// Flattens the channel tree and its clients in display order.
pub fn tree_rows(
    server: &ServerInfo,
    channels: &[Channel],
    clients: &[Client],
    me: Option<ClientId>,
    collapsed: &BTreeSet<ChannelId>,
) -> Vec<Row> {
    let mut rows = vec![Row::Server {
        name: server.name.clone(),
    }];
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
            rows.push(Row::Channel {
                id: ch.id,
                name: ch.name.clone(),
                depth,
                full: ch.max_clients > 0 && inside.len() as u32 >= ch.max_clients,
                is_default: ch.is_default,
                spacer: ch.name.starts_with("[spacer"),
                count: inside.len(),
                has_children,
                collapsed,
            });
            if collapsed {
                continue;
            }
            for c in inside {
                rows.push(Row::Client {
                    id: c.id,
                    name: c.name.clone(),
                    depth: depth + 1,
                    muted: c.muted,
                    deaf: c.deaf,
                    away: c.away,
                    me: Some(c.id) == cx.me,
                    sharing: c.sharing,
                    role: c.role,
                });
            }
            walk(cx, Some(ch.id), depth + 1, rows);
        }
    }
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

    #[test]
    fn tree_is_depth_first_with_clients_before_subchannels() {
        let channels = vec![
            ch(1, "Lobby", None, 0),
            ch(2, "Gaming", None, 1),
            ch(3, "CS", Some(2), 0),
        ];
        let clients = vec![cl(10, "bob", 2), cl(11, "Alice", 2), cl(12, "carl", 3)];
        let rows = tree_rows(
            &ServerInfo::default(),
            &channels,
            &clients,
            Some(11),
            &BTreeSet::new(),
        );
        let names: Vec<String> = rows
            .iter()
            .map(|r| match r {
                Row::Server { .. } => "S".into(),
                Row::Channel { name, depth, .. } => format!("{depth}#{name}"),
                Row::Client {
                    name, depth, me, ..
                } => format!("{depth}@{name}{}", if *me { "*" } else { "" }),
            })
            .collect();
        assert_eq!(
            names,
            [
                "S", "1#Lobby", "1#Gaming", "2@Alice*", "2@bob", "2#CS", "3@carl"
            ]
        );
    }
}
