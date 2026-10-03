//! Interface sounds: what each cue sounds like, the synthesis that renders it, and the player
//! that mixes it into the speakers.

mod bank;
mod kind;
mod player;
mod synth;

pub use kind::Cue;
pub use player::CuePlayer;

/// Renders every cue ahead of the first play, so the audio thread never synthesizes.
pub fn prepare() {
    bank::prepare();
}
