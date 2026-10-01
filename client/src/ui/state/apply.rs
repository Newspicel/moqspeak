//! Routes engine events to signals: state to the tree, messages to the conversations, and what
//! happened to the log.

use std::collections::BTreeSet;

use zgui::prelude::*;

use crate::engine::{ConnStatus, Event, MediaStatus};
use crate::model::{ChatTarget, ClientMsg};
use crate::ui::state::app::AppState;
use crate::ui::state::chat::{Conversation, MessageKind};
use crate::ui::state::log::Tone;
use crate::ui::state::note::{Level, Note};
use crate::ui::state::pointer::Selection;

impl AppState {
    /// Applies one event from the network.
    pub fn apply(&self, event: Event) {
        match event {
            Event::Status(status) => self.apply_status(status),
            Event::Welcome { you } => {
                self.me.set(Some(you.id));
                self.log_line(Tone::Info, format!("Connected as {}", you.name));
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
                    let welcome = server.welcome.clone();
                    self.chat
                        .update(|c| c.push(Conversation::Server, MessageKind::Note, welcome));
                }
                self.chat.update(|chat| {
                    for (id, name) in chat.direct.iter_mut() {
                        if let Some(c) = clients.iter().find(|c| c.id == *id) {
                            name.clone_from(&c.name);
                        }
                    }
                });
                let gone = match self.selected.get_untracked() {
                    Selection::Client(id) => !clients.iter().any(|c| c.id == id),
                    Selection::Channel(id) => !channels.iter().any(|c| c.id == id),
                    Selection::None => false,
                };
                if gone {
                    self.selected.set(Selection::None);
                }
                // A development aid for screenshots: MOQSPEAK_SELECT=<nickname>.
                if let Ok(wanted) = std::env::var("MOQSPEAK_SELECT")
                    && self.selected.get_untracked() == Selection::None
                    && let Some(c) = clients.iter().find(|c| c.name == wanted)
                {
                    self.selected.set(Selection::Client(c.id));
                }
                self.server.set(server);
                self.channels.set(channels);
                self.clients.set(clients);
            }
            Event::Log { error, text } => {
                if error {
                    self.fail(&text, None);
                } else {
                    self.log_line(Tone::Event, text);
                }
            }
            Event::Chat {
                target,
                from,
                from_name,
                to,
                text,
            } => {
                let own = Some(from) == self.me.get_untracked();
                let conversation = match target {
                    ChatTarget::Server => Conversation::Server,
                    ChatTarget::Channel => Conversation::Channel,
                    ChatTarget::Client => {
                        let peer = if own { to.unwrap_or(from) } else { from };
                        let name = self
                            .client(peer)
                            .map(|c| c.name)
                            .unwrap_or(from_name.clone());
                        self.chat.update(|c| c.add_direct(peer, name));
                        Conversation::Private(peer)
                    }
                };
                self.chat.update(|c| {
                    c.push(
                        conversation,
                        MessageKind::Text {
                            from: from_name,
                            own,
                        },
                        text,
                    );
                });
            }
            Event::Poke {
                from,
                from_name,
                text,
                ..
            } => {
                self.log_line(Tone::Event, format!("{from_name} poked you"));
                self.note(Note {
                    level: Level::Info,
                    title: format!("{from_name} poked you"),
                    detail: (!text.is_empty()).then_some(text),
                    reply: Some(from),
                });
            }
            Event::Talking(set) => self.talking.set(set),
            Event::Sharing(on) => self.sharing.set(on),
            Event::Media(status) => {
                let was = matches!(self.media.get_untracked(), MediaStatus::Connected { .. });
                match &status {
                    MediaStatus::Connected { .. } if !was => {
                        self.log_line(Tone::Info, "Voice connected");
                    }
                    MediaStatus::Failed(e) => {
                        self.log_line(Tone::Error, format!("Voice unavailable: {e}"));
                    }
                    _ => {}
                }
                self.media.set(status);
            }
        }
    }

    fn apply_status(&self, status: ConnStatus) {
        match &status {
            ConnStatus::Connected => self.connected_at.set(Some(std::time::Instant::now())),
            ConnStatus::Connecting(_) => {}
            ConnStatus::Disconnected | ConnStatus::Failed(_) => {
                if let ConnStatus::Failed(e) = &status {
                    self.log_line(Tone::Error, format!("Connection failed: {e}"));
                } else if self.connected_at.get_untracked().is_some() {
                    self.log_line(Tone::Info, "Disconnected");
                }
                self.connected_at.set(None);
                self.clients.set(Vec::new());
                self.channels.set(Vec::new());
                self.me.set(None);
                self.talking.set(BTreeSet::new());
                self.selected.set(Selection::None);
            }
        }
        self.status.set(status);
    }
}
