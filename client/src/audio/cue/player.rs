//! The cues sounding right now, mixed sample by sample into the speaker feed.

use super::bank;
use super::kind::Cue;

/// The most cues that sound at once. A new cue beyond this ends the oldest.
const VOICES: usize = 6;

/// Plays cues from the start to the end of their samples.
#[derive(Default)]
pub struct CuePlayer {
    playing: Vec<(&'static [f32], usize)>,
}

impl CuePlayer {
    /// Starts `cue`. The same cue already playing starts over.
    pub fn play(&mut self, cue: Cue) {
        let samples = bank::samples(cue);
        self.playing.retain(|(s, _)| !std::ptr::eq(*s, samples));
        if self.playing.len() >= VOICES {
            self.playing.remove(0);
        }
        self.playing.push((samples, 0));
    }

    /// Adds the next `out.len()` samples of every playing cue into `out`.
    pub fn mix_into(&mut self, out: &mut [f32]) {
        for (samples, pos) in &mut self.playing {
            let rest = &samples[*pos..];
            let n = rest.len().min(out.len());
            for (o, s) in out.iter_mut().zip(&rest[..n]) {
                *o += s;
            }
            *pos += n;
        }
        self.playing.retain(|(s, pos)| *pos < s.len());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plays_to_the_end_and_stops() {
        let mut player = CuePlayer::default();
        player.play(Cue::TalkOn);
        let len = bank::samples(Cue::TalkOn).len();
        let mut out = vec![0.0; len + 100];
        player.mix_into(&mut out);
        assert!(out[..len].iter().any(|s| s.abs() > 0.05));
        assert!(out[len..].iter().all(|s| *s == 0.0));
        assert!(player.playing.is_empty());
    }

    #[test]
    fn restarts_a_repeated_cue() {
        let mut player = CuePlayer::default();
        player.play(Cue::Mute);
        player.mix_into(&mut [0.0; 64]);
        player.play(Cue::Mute);
        assert_eq!(player.playing.len(), 1);
        assert_eq!(player.playing[0].1, 0);
    }
}
