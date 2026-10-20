//! Voice frames between TeamSpeak and the mixer. TeamSpeak counts its voice packets in 16 bits
//! and wraps, while the mixer reads a sequence that only grows, and conceals a short gap in it.
//! [`Spurts`] widens the one into the other, per speaker.

use std::collections::HashMap;

use crate::model::ClientId;

/// The widened sequence of every speaker heard so far.
#[derive(Default)]
pub struct Spurts(HashMap<ClientId, Spurt>);

impl Spurts {
    /// The sequence `id` stands for in a sequence that only grows.
    pub fn widen(&mut self, client: ClientId, id: u16) -> u32 {
        self.0
            .entry(client)
            .or_insert(Spurt { last: id, epoch: 0 })
            .widen(id)
    }

    /// Drops a speaker, so that a client that returns starts over.
    pub fn forget(&mut self, client: ClientId) {
        self.0.remove(&client);
    }
}

/// One speaker's place in the 16-bit count.
struct Spurt {
    last: u16,
    epoch: u32,
}

impl Spurt {
    fn widen(&mut self, id: u16) -> u32 {
        // The shorter way round the circle tells us whether this frame is new or late.
        let ahead = id.wrapping_sub(self.last);
        let behind = self.last.wrapping_sub(id);
        if ahead <= behind {
            if id < self.last {
                self.epoch = self.epoch.wrapping_add(1);
            }
            self.last = id;
            seq(self.epoch, id)
        } else {
            // A late frame belongs to this turn of the circle, or the one before it.
            let epoch = if id > self.last {
                self.epoch.wrapping_sub(1)
            } else {
                self.epoch
            };
            seq(epoch, id)
        }
    }
}

fn seq(epoch: u32, id: u16) -> u32 {
    (epoch << 16) | id as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_at_the_first_frame_it_sees() {
        let mut s = Spurts::default();
        assert_eq!(s.widen(1, 7), 7);
        assert_eq!(s.widen(1, 8), 8);
    }

    #[test]
    fn keeps_a_gap_a_gap() {
        let mut s = Spurts::default();
        s.widen(1, 10);
        // Two frames lost: the mixer must see a gap of two to conceal them.
        assert_eq!(s.widen(1, 13) - 10, 3);
    }

    #[test]
    fn grows_across_the_wrap() {
        let mut s = Spurts::default();
        s.widen(1, 65534);
        assert_eq!(s.widen(1, 65535), 65535);
        // The next frame wraps to zero, and the sequence must still step by one.
        assert_eq!(s.widen(1, 0), 65536);
        assert_eq!(s.widen(1, 1), 65537);
    }

    #[test]
    fn places_a_late_frame_behind_the_one_that_passed_it() {
        let mut s = Spurts::default();
        s.widen(1, 100);
        let ahead = s.widen(1, 102);
        let late = s.widen(1, 101);
        assert_eq!(ahead, 102);
        assert_eq!(late, 101);
        // The late frame must not move the count on.
        assert_eq!(s.widen(1, 103), 103);
    }

    #[test]
    fn places_a_late_frame_from_before_the_wrap() {
        let mut s = Spurts::default();
        s.widen(1, 65535);
        assert_eq!(s.widen(1, 1), 65537);
        // This one was sent before the wrap and arrived after it.
        assert_eq!(s.widen(1, 65534), 65534);
    }

    #[test]
    fn counts_each_speaker_on_its_own() {
        let mut s = Spurts::default();
        assert_eq!(s.widen(1, 500), 500);
        assert_eq!(s.widen(2, 9), 9);
        assert_eq!(s.widen(1, 501), 501);
    }

    #[test]
    fn a_speaker_that_returns_starts_over() {
        let mut s = Spurts::default();
        s.widen(1, 65535);
        s.widen(1, 0);
        s.forget(1);
        assert_eq!(s.widen(1, 42), 42);
    }
}
