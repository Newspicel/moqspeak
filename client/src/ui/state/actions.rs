//! What the person can do: connect, talk, move, chat and choose.

use std::sync::atomic::Ordering;

use crate::audio::VoiceMode;
use crate::engine::Command;
use crate::model::{ChannelId, ChatTarget, ClientId, ClientMsg};
use crate::ui::state::app::AppState;
use crate::ui::state::chat::Conversation;
use crate::ui::state::log::Tone;
use crate::ui::state::note::{Level, Note};
use crate::ui::state::pointer::Selection;
use crate::ui::state::settings::{Bookmark, mode_to_str};
use crate::ui::theme::{Scheme, Variant};
use zgui::prelude::*;

impl AppState {
    pub fn connect(&self, address: String, nickname: String) {
        self.update_settings(|s| {
            s.address = address.clone();
            s.nickname = nickname.clone();
        });
        self.send(Command::Connect { address, nickname });
    }

    /// Connects with the nickname a bookmark carries, or the saved one.
    pub fn connect_bookmark(&self, bookmark: &Bookmark) {
        let nickname = if bookmark.nickname.is_empty() {
            self.settings.with_untracked(|s| s.nickname.clone())
        } else {
            bookmark.nickname.clone()
        };
        self.connect(bookmark.address.clone(), nickname);
    }

    pub fn disconnect(&self) {
        self.send(Command::Disconnect);
    }

    /// Saves the current server, unless it is saved already.
    pub fn add_bookmark(&self) {
        let label = self.server_label();
        self.update_settings(|s| {
            if s.bookmark_of(&s.address).is_none() {
                s.bookmarks.push(Bookmark {
                    label,
                    address: s.address.clone(),
                    nickname: String::new(),
                });
            }
        });
    }

    pub fn remove_bookmark(&self, address: &str) {
        self.update_settings(|s| s.bookmarks.retain(|b| b.address != address));
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

    /// Plays the microphone back to the speakers, or stops.
    pub fn set_loopback(&self, on: bool) {
        self.loopback.set(on);
        self.engine
            .with_value(|e| e.audio.shared.loopback.store(on, Ordering::Relaxed));
    }

    /// Records a press on `target` and says whether it completes a double press.
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

    /// Logs an error and announces it.
    pub fn fail(&self, title: &str, detail: Option<String>) {
        let line = match &detail {
            Some(d) => format!("{title}: {d}"),
            None => title.to_owned(),
        };
        self.log_line(Tone::Error, line);
        self.note(Note {
            level: Level::Error,
            title: title.to_owned(),
            detail,
            reply: None,
        });
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
            if drag.active
                && let Some(target) = self.drop_target.get_untracked()
            {
                self.move_client(drag.client, target);
            }
            self.drag.set(None);
        }
        if self.drop_target.get_untracked().is_some() {
            self.drop_target.set(None);
        }
    }

    /// Drops a drag without moving anybody.
    pub fn cancel_drag(&self) {
        self.drag.set(None);
        self.drop_target.set(None);
    }

    /// Whether a drag is under way.
    pub fn dragging(&self) -> bool {
        self.drag.with(|d| d.as_ref().is_some_and(|d| d.active))
    }

    pub fn toggle_collapsed(&self, channel: ChannelId) {
        self.collapsed.update(|c| {
            if !c.remove(&channel) {
                c.insert(channel);
            }
        });
    }

    pub fn set_scheme(&self, scheme: Scheme) {
        self.scheme.set(scheme);
        self.update_settings(|s| s.theme_mode = scheme.name().into());
    }

    pub fn set_variant(&self, variant: Variant) {
        self.variant.set(variant);
        self.update_settings(|s| s.theme = variant.name().into());
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

    /// Opens a private conversation with `id`.
    pub fn open_direct(&self, id: ClientId) {
        if Some(id) == self.me.get_untracked() {
            return;
        }
        let Some(client) = self.client(id) else {
            return;
        };
        self.chat.update(|c| c.open_direct(id, client.name));
    }

    /// Sends `text` to the conversation the panel shows.
    pub fn send_chat(&self, text: String) {
        let text = text.trim().to_owned();
        if text.is_empty() {
            return;
        }
        let (target, to) = match self.chat.with_untracked(|c| c.current) {
            Conversation::Server => (ChatTarget::Server, None),
            Conversation::Channel => (ChatTarget::Channel, None),
            Conversation::Private(id) => (ChatTarget::Client, Some(id)),
        };
        self.msg(ClientMsg::Chat { target, to, text });
    }
}
