//! The cues the interface plays and the notes each one strikes.

use super::synth::{GLASS, SOFT, Sound, Strike};

/// Something the interface announces with a sound.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Cue {
    Mute,
    Unmute,
    Deafen,
    Undeafen,
    /// Someone enters your channel.
    PeerJoin,
    /// Someone leaves your channel.
    PeerLeave,
    /// You arrive in another channel.
    Move,
    Connect,
    Disconnect,
    Poke,
    /// A private message from someone else.
    Message,
    TalkOn,
    TalkOff,
    #[cfg(feature = "screen-share")]
    ShareOn,
    #[cfg(feature = "screen-share")]
    ShareOff,
}

impl Cue {
    /// Every cue, in a fixed order.
    pub const ALL: &[Cue] = &[
        Cue::Mute,
        Cue::Unmute,
        Cue::Deafen,
        Cue::Undeafen,
        Cue::PeerJoin,
        Cue::PeerLeave,
        Cue::Move,
        Cue::Connect,
        Cue::Disconnect,
        Cue::Poke,
        Cue::Message,
        Cue::TalkOn,
        Cue::TalkOff,
        #[cfg(feature = "screen-share")]
        Cue::ShareOn,
        #[cfg(feature = "screen-share")]
        Cue::ShareOff,
    ];

    /// The position of this cue in [`Cue::ALL`].
    pub(super) fn index(self) -> usize {
        Self::ALL.iter().position(|c| *c == self).unwrap_or(0)
    }

    /// What this cue sounds like. Rising intervals turn something on, falling ones turn it off.
    pub(super) fn sound(self) -> Sound {
        const fn s(at: f32, note: f32, decay: f32) -> Strike {
            Strike::new(at, note, decay)
        }
        let soft = |strikes: Vec<Strike>, peak, room| Sound {
            partials: SOFT,
            strikes,
            peak,
            room,
        };
        let glass = |strikes: Vec<Strike>, peak, room| Sound {
            partials: GLASS,
            strikes,
            peak,
            room,
        };
        match self {
            // A fourth around A5.
            Cue::Unmute => soft(
                vec![s(0.0, 76.0, 0.1), s(0.07, 81.0, 0.12).bend(-0.6)],
                0.3,
                0.18,
            ),
            Cue::Mute => soft(
                vec![s(0.0, 81.0, 0.1), s(0.07, 76.0, 0.12).bend(0.6)],
                0.3,
                0.18,
            ),
            // An octave on D, longer than the microphone.
            Cue::Undeafen => soft(
                vec![s(0.0, 74.0, 0.12), s(0.08, 86.0, 0.16).bend(-0.8)],
                0.32,
                0.2,
            ),
            Cue::Deafen => soft(
                vec![s(0.0, 86.0, 0.12), s(0.08, 74.0, 0.16).bend(0.8)],
                0.32,
                0.2,
            ),
            // A fifth on C that swoops into each note.
            Cue::PeerJoin => soft(
                vec![
                    s(0.0, 72.0, 0.11).bend(-2.0),
                    s(0.075, 79.0, 0.15).bend(-2.0),
                ],
                0.28,
                0.22,
            ),
            Cue::PeerLeave => soft(
                vec![s(0.0, 79.0, 0.11).bend(2.0), s(0.075, 72.0, 0.15).bend(2.0)],
                0.26,
                0.22,
            ),
            // A quick rising triad.
            Cue::Move => soft(
                vec![
                    s(0.0, 72.0, 0.1),
                    s(0.05, 76.0, 0.1),
                    s(0.1, 79.0, 0.16).bend(-0.5),
                ],
                0.28,
                0.24,
            ),
            // A rising C major arpeggio with a long ring.
            Cue::Connect => soft(
                vec![
                    s(0.0, 72.0, 0.18),
                    s(0.07, 76.0, 0.18),
                    s(0.14, 79.0, 0.18),
                    s(0.21, 84.0, 0.24).gain(0.9),
                ],
                0.3,
                0.3,
            ),
            Cue::Disconnect => soft(
                vec![
                    s(0.0, 79.0, 0.16),
                    s(0.08, 76.0, 0.16),
                    s(0.16, 72.0, 0.26).bend(0.5),
                ],
                0.28,
                0.3,
            ),
            // Two knocks on a high bell.
            Cue::Poke => glass(
                vec![s(0.0, 93.0, 0.14), s(0.13, 93.0, 0.22).gain(0.9)],
                0.3,
                0.22,
            ),
            // A bright ding that rises a fifth.
            Cue::Message => glass(
                vec![s(0.0, 88.0, 0.12).gain(0.7), s(0.05, 95.0, 0.2)],
                0.26,
                0.24,
            ),
            // Short blips with no room, quiet enough to repeat all day.
            Cue::TalkOn => soft(vec![s(0.0, 86.0, 0.035).bend(-1.5)], 0.15, 0.0),
            Cue::TalkOff => soft(vec![s(0.0, 81.0, 0.035).bend(1.5)], 0.13, 0.0),
            #[cfg(feature = "screen-share")]
            Cue::ShareOn => soft(
                vec![
                    s(0.0, 79.0, 0.12),
                    s(0.06, 86.0, 0.12),
                    s(0.12, 91.0, 0.2).gain(0.8),
                ],
                0.28,
                0.26,
            ),
            #[cfg(feature = "screen-share")]
            Cue::ShareOff => soft(
                vec![
                    s(0.0, 91.0, 0.12).gain(0.8),
                    s(0.06, 86.0, 0.12),
                    s(0.12, 79.0, 0.2),
                ],
                0.26,
                0.26,
            ),
        }
    }
}
