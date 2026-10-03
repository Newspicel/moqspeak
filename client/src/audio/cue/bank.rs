//! The rendered samples of every cue, made once per process.

use std::sync::LazyLock;

use super::kind::Cue;
use super::synth;

static BANK: LazyLock<Vec<Vec<f32>>> =
    LazyLock::new(|| Cue::ALL.iter().map(|c| synth::render(&c.sound())).collect());

/// Renders the bank now.
pub fn prepare() {
    LazyLock::force(&BANK);
}

/// The samples of `cue` at the codec rate.
pub fn samples(cue: Cue) -> &'static [f32] {
    &BANK[cue.index()]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::RATE;

    #[test]
    fn every_cue_is_short_and_clean() {
        for &cue in Cue::ALL {
            let s = samples(cue);
            let peak = s.iter().fold(0.0f32, |m, v| m.max(v.abs()));
            assert!(peak > 0.1 && peak <= 0.35, "{cue:?} peaks at {peak}");
            assert!(
                s.len() < RATE as usize * 2,
                "{cue:?} lasts {} samples",
                s.len()
            );
            assert!(s[0].abs() < 1e-3, "{cue:?} starts with a click");
        }
    }
}
