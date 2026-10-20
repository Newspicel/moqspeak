//! What the person can do: connect, talk, move, chat and choose.

use std::sync::atomic::Ordering;

use crate::audio::{Cue, VoiceMode};
use crate::engine::Command;
use crate::model::{ChannelId, ChatTarget, ClientId};
use crate::transport::Action;
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

    /// Remembers the server just joined, newest first and each address once.
    pub fn remember_server(&self, name: &str) {
        /// How many servers the history keeps.
        const KEEP: usize = 8;
        let (address, nickname) = self
            .settings
            .with_untracked(|s| (s.address.clone(), s.nickname.clone()));
        if address.is_empty() {
            return;
        }
        let label = if name.is_empty() {
            address.clone()
        } else {
            name.to_owned()
        };
        self.update_settings(|s| {
            s.history.retain(|h| h.address != address);
            s.history.insert(
                0,
                Bookmark {
                    label,
                    address: address.clone(),
                    nickname: nickname.clone(),
                },
            );
            s.history.truncate(KEEP);
        });
    }

    /// Saves a server from the history, unless it is saved already.
    pub fn save_server(&self, label: &str, address: &str) {
        self.update_settings(|s| {
            if s.bookmark_of(address).is_none() {
                s.bookmarks.push(Bookmark {
                    label: label.to_owned(),
                    address: address.to_owned(),
                    nickname: String::new(),
                });
            }
        });
    }

    /// Drops one server from the history.
    pub fn forget_server(&self, address: &str) {
        self.update_settings(|s| s.history.retain(|h| h.address != address));
    }

    pub fn join(&self, channel: ChannelId) {
        if self.my_client().is_some_and(|c| c.channel == channel) {
            return;
        }
        self.msg(Action::Join { channel });
    }

    pub fn set_mic_muted(&self, muted: bool) {
        self.mic_choice.set_value(muted);
        // The sound being off holds the microphone down, so asking for the microphone back asks
        // for the sound back with it.
        if !muted && self.deafened.get_untracked() {
            self.set_deafened(false);
            return;
        }
        self.mic_muted.set(muted);
        self.cue(if muted { Cue::Mute } else { Cue::Unmute });
        self.engine
            .with_value(|e| e.audio.shared.mic_muted.store(muted, Ordering::Relaxed));
        self.msg(Action::Status {
            muted: Some(muted),
            deaf: None,
            away: None,
            away_message: None,
        });
    }

    /// Turns the sound off or on. It takes the microphone with it, because somebody who hears
    /// nothing cannot answer, and gives back the microphone it found.
    pub fn set_deafened(&self, deaf: bool) {
        self.deafened.set(deaf);
        self.cue(if deaf { Cue::Deafen } else { Cue::Undeafen });
        let muted = mic_muted(self.mic_choice.get_value(), deaf);
        self.mic_muted.set(muted);
        self.engine.with_value(|e| {
            e.audio.shared.deafened.store(deaf, Ordering::Relaxed);
            e.audio.shared.mic_muted.store(muted, Ordering::Relaxed);
        });
        // One message carries both, so a server never sees a half-changed client.
        self.msg(Action::Status {
            muted: Some(muted),
            deaf: Some(deaf),
            away: None,
            away_message: None,
        });
    }

    pub fn set_away(&self, away: bool) {
        self.away.set(away);
        self.msg(Action::Status {
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
            if self.voice_mode.get_untracked() == VoiceMode::PushToTalk {
                self.cue(if down { Cue::TalkOn } else { Cue::TalkOff });
            }
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
            self.msg(Action::Move {
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
        self.msg(Action::Chat { target, to, text });
    }
}

/// Whether the microphone is muted, from what the person chose for it and whether the sound is
/// off. The sound being off holds the microphone down, whatever the choice was.
pub(super) fn mic_muted(choice: bool, deafened: bool) -> bool {
    deafened || choice
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sound_going_off_takes_the_microphone_with_it() {
        assert!(mic_muted(false, true));
        assert!(mic_muted(true, true));
    }

    #[test]
    fn the_sound_coming_back_gives_back_the_choice() {
        assert!(!mic_muted(false, false));
        // Somebody who muted the microphone first keeps it muted.
        assert!(mic_muted(true, false));
    }
}
