//! The interface state: what the network reported and what the person chose, as signals.

use std::collections::{BTreeSet, HashMap};
use std::sync::atomic::Ordering;

use zgui::prelude::*;
use zgui::reactive::StoredValue;

use crate::audio::VoiceMode;
use crate::engine::{Command, Engine};
use crate::model::{Channel, ChannelId, Client, ClientId, Role, ServerInfo};
use crate::transport::{Action, Capabilities, ConnStatus, MediaStatus};
use crate::ui::state::chat::Chat;
use crate::ui::state::log::{Log, Tone};
use crate::ui::state::modal::Modal;
use crate::ui::state::note::Note;
use crate::ui::state::pointer::{Drag, Selection};
use crate::ui::state::settings::{CHAT_MAX, CHAT_MIN, Settings, mode_from_str};
use crate::ui::theme::{Scheme, Variant};

/// Every signal the interface reads. Provided as local context at the root.
#[derive(Clone, Copy)]
pub struct AppState {
    pub engine: StoredValue<Engine>,
    pub status: RwSignal<ConnStatus>,
    pub media: RwSignal<MediaStatus>,
    /// What the live transport can do. The interface offers only these.
    pub caps: RwSignal<Capabilities>,
    pub server: RwSignal<ServerInfo>,
    pub channels: RwSignal<Vec<Channel>>,
    pub clients: RwSignal<Vec<Client>>,
    pub me: RwSignal<Option<ClientId>>,
    pub talking: RwSignal<BTreeSet<ClientId>>,
    pub chat: RwSignal<Chat>,
    pub log: RwSignal<Log>,
    pub log_open: RwSignal<bool>,
    pub notes: RwSignal<Vec<Note>>,
    pub selected: RwSignal<Selection>,
    pub settings: RwSignal<Settings>,
    pub settings_open: RwSignal<bool>,
    /// The settings section shown, kept while the window stands.
    pub settings_page: RwSignal<String>,
    pub modal: RwSignal<Modal>,
    pub mic_muted: RwSignal<bool>,
    pub deafened: RwSignal<bool>,
    pub away: RwSignal<bool>,
    pub loopback: RwSignal<bool>,
    pub voice_mode: RwSignal<VoiceMode>,
    pub scheme: RwSignal<Scheme>,
    pub variant: RwSignal<Variant>,
    #[cfg(feature = "screen-share")]
    pub sharing: RwSignal<bool>,
    /// How wide the chat column stands open, in CSS pixels.
    pub chat_width: RwSignal<f32>,
    pub drag: RwSignal<Option<Drag>>,
    pub drop_target: RwSignal<Option<ChannelId>>,
    pub pointer: RwSignal<(f32, f32)>,
    pub collapsed: RwSignal<BTreeSet<ChannelId>>,
    pub(super) last_click: StoredValue<Option<(Selection, std::time::Instant)>>,
    /// The application root's owner. A window opened from deep inside the tree opens under it,
    /// because a window's handle state belongs to the owner `open` runs in.
    #[cfg(feature = "screen-share")]
    pub root: StoredValue<zgui::reactive::Owner>,
    pub ptt: RwSignal<bool>,
    pub local_mutes: RwSignal<BTreeSet<ClientId>>,
    pub volumes: RwSignal<HashMap<ClientId, f32>>,
    pub connected_at: RwSignal<Option<std::time::Instant>>,
}

impl AppState {
    pub fn new(engine: Engine) -> Self {
        let settings = Settings::load();
        let mode = mode_from_str(&settings.voice_mode);
        let chat_width = settings.chat_width.clamp(CHAT_MIN, CHAT_MAX);
        let audio = &engine.audio.shared;
        audio.set_mode(mode);
        audio.threshold_db.set(settings.threshold_db);
        audio.input_gain.set(settings.input_gain);
        audio.master_volume.set(settings.output_volume);
        audio.cue_volume.set(settings.sound_volume);
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
            caps: RwSignal::new(Capabilities::NONE),
            server: RwSignal::new(ServerInfo::default()),
            channels: RwSignal::new(Vec::new()),
            clients: RwSignal::new(Vec::new()),
            me: RwSignal::new(None),
            talking: RwSignal::new(BTreeSet::new()),
            chat: RwSignal::new(Chat::default()),
            log: RwSignal::new(Log::default()),
            log_open: RwSignal::new(false),
            notes: RwSignal::new(Vec::new()),
            selected: RwSignal::new(Selection::None),
            scheme: RwSignal::new(Scheme::parse(&settings.theme_mode)),
            variant: RwSignal::new(Variant::from_name(&settings.theme).unwrap_or_default()),
            settings: RwSignal::new(settings),
            settings_open: RwSignal::new(false),
            settings_page: RwSignal::new("voice".to_owned()),
            modal: RwSignal::new(Modal::None),
            mic_muted: RwSignal::new(false),
            deafened: RwSignal::new(false),
            away: RwSignal::new(false),
            loopback: RwSignal::new(false),
            voice_mode: RwSignal::new(mode),
            #[cfg(feature = "screen-share")]
            sharing: RwSignal::new(false),
            chat_width: RwSignal::new(chat_width),
            drag: RwSignal::new(None),
            drop_target: RwSignal::new(None),
            pointer: RwSignal::new((0.0, 0.0)),
            collapsed: RwSignal::new(BTreeSet::new()),
            last_click: StoredValue::new(None),
            #[cfg(feature = "screen-share")]
            root: StoredValue::new(
                zgui::reactive::Owner::current()
                    .expect("AppState is created inside the app's root owner"),
            ),
            ptt: RwSignal::new(false),
            local_mutes: RwSignal::new(BTreeSet::new()),
            volumes: RwSignal::new(HashMap::new()),
            connected_at: RwSignal::new(None),
        }
    }

    pub fn expect() -> Self {
        use_local_context::<Self>().expect("AppState is provided at the root")
    }

    pub fn send(&self, command: Command) {
        // During shutdown the state can be gone before a window's cleanup runs.
        let _ = self.engine.try_with_value(|e| e.send(command));
    }

    pub fn msg(&self, msg: Action) {
        self.send(Command::Send(msg));
    }

    pub fn connected(&self) -> bool {
        self.status.with(|s| *s == ConnStatus::Connected)
    }

    /// Whether the window shows a server: connected, or still holding the tree of one.
    pub fn online(&self) -> bool {
        self.connected() || !self.channels.with(Vec::is_empty)
    }

    pub fn my_client(&self) -> Option<Client> {
        let me = self.me.get()?;
        self.clients
            .with(|c| c.iter().find(|c| c.id == me).cloned())
    }

    /// What this client may do on the server it is connected to.
    pub fn my_role(&self) -> Role {
        self.my_client().map(|c| c.role).unwrap_or_default()
    }

    /// Whether the live transport creates, edits and deletes channels.
    pub fn can_channels(&self) -> bool {
        self.caps.with(|c| c.channels)
    }

    /// Whether the live transport assigns the roles this client knows.
    pub fn can_roles(&self) -> bool {
        self.caps.with(|c| c.roles)
    }

    /// Whether the live transport removes a client from the server.
    pub fn can_kick(&self) -> bool {
        self.caps.with(|c| c.kick)
    }

    /// Whether to offer what a moderator does. A transport that carries roles gates on mine. One
    /// that does not leaves the decision to the server, which answers with an error where it
    /// refuses.
    pub fn may_moderate(&self) -> bool {
        !self.can_roles() || self.my_role() >= Role::Mod
    }

    /// Whether to offer screen sharing. The key stays while offline, so the pill keeps its shape.
    pub fn offers_share(&self) -> bool {
        !self.connected() || self.caps.with(|c| c.screen_share)
    }

    pub fn my_role_untracked(&self) -> Role {
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

    /// The channel you are in.
    pub fn my_channel(&self) -> Option<Channel> {
        self.my_client().and_then(|c| self.channel(c.channel))
    }

    /// The name the head shows for the server.
    pub fn server_label(&self) -> String {
        let name = self.server.with(|s| s.name.clone());
        if !name.is_empty() {
            return name;
        }
        let address = self.settings.with(|s| s.address.clone());
        self.settings
            .with(|s| s.bookmark_of(&address).map(|b| b.label.clone()))
            .unwrap_or(address)
    }

    /// Adds a line to the server log.
    pub fn log_line(&self, tone: Tone, text: impl Into<String>) {
        self.log.update(|log| log.push(tone, text.into()));
    }

    /// Shows an announcement in the corner.
    pub fn note(&self, note: Note) {
        self.notes.update(|n| n.push(note));
    }

    pub fn update_settings(&self, f: impl FnOnce(&mut Settings)) {
        self.settings.update(f);
        self.settings.with_untracked(Settings::save);
    }
}
