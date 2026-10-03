//! Which interface sound a change calls for, and the call that plays it.

use zgui::prelude::*;

use crate::audio::Cue;
use crate::model::{Client, ClientId};
use crate::ui::state::app::AppState;

impl AppState {
    /// Plays `cue` unless the person turned the sounds off.
    pub fn cue(&self, cue: Cue) {
        if self.settings.with_untracked(|s| s.sounds) {
            let _ = self.engine.try_with_value(|e| e.audio.mixer.cue(cue));
        }
    }
}

/// The cue a new client list calls for, seen from `me`. A move of your own wins over others
/// arriving, and arrivals win over departures.
pub fn tree_cue(before: &[Client], after: &[Client], me: ClientId) -> Option<Cue> {
    let channel_of = |list: &[Client], id| list.iter().find(|c| c.id == id).map(|c| c.channel);
    let (was, now) = (channel_of(before, me)?, channel_of(after, me)?);
    if was != now {
        return Some(Cue::Move);
    }
    let arrived = |from: &[Client], to: &[Client]| {
        to.iter()
            .any(|c| c.id != me && c.channel == now && channel_of(from, c.id) != Some(now))
    };
    if arrived(before, after) {
        Some(Cue::PeerJoin)
    } else if arrived(after, before) {
        Some(Cue::PeerLeave)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn client(id: ClientId, channel: u64) -> Client {
        Client {
            id,
            uid: String::new(),
            name: format!("c{id}"),
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
    fn your_move_wins() {
        let before = [client(1, 10), client(2, 20)];
        let after = [client(1, 20), client(2, 20)];
        assert_eq!(tree_cue(&before, &after, 1), Some(Cue::Move));
    }

    #[test]
    fn someone_arrives() {
        let before = [client(1, 10), client(2, 20)];
        let after = [client(1, 10), client(2, 10)];
        assert_eq!(tree_cue(&before, &after, 1), Some(Cue::PeerJoin));
        let connected = [client(1, 10), client(2, 20), client(3, 10)];
        assert_eq!(tree_cue(&before, &connected, 1), Some(Cue::PeerJoin));
    }

    #[test]
    fn someone_leaves() {
        let before = [client(1, 10), client(2, 10)];
        assert_eq!(
            tree_cue(&before, &[client(1, 10), client(2, 30)], 1),
            Some(Cue::PeerLeave)
        );
        assert_eq!(tree_cue(&before, &[client(1, 10)], 1), Some(Cue::PeerLeave));
    }

    #[test]
    fn other_channels_stay_quiet() {
        let before = [client(1, 10), client(2, 20)];
        let after = [client(1, 10), client(2, 30), client(3, 20)];
        assert_eq!(tree_cue(&before, &after, 1), None);
    }

    #[test]
    fn the_first_list_stays_quiet() {
        assert_eq!(tree_cue(&[], &[client(1, 10), client(2, 10)], 1), None);
    }
}
