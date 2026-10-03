//! Additive synthesis of short struck tones: a few partials with a pitch glide, an exponential
//! decay and a small room.

use std::f64::consts::TAU;

use crate::audio::RATE;

/// The partials of one instrument as (frequency ratio, amplitude, decay scale).
pub type Partials = &'static [(f32, f32, f32)];

/// A round mallet tone. A slightly detuned copy of the fundamental gives it a slow shimmer.
pub const SOFT: Partials = &[
    (1.0, 1.0, 1.0),
    (1.0026, 0.3, 1.0),
    (2.0, 0.16, 0.5),
    (3.0, 0.05, 0.3),
    (4.0, 0.02, 0.2),
];

/// A small glass bell with inharmonic overtones.
pub const GLASS: Partials = &[
    (1.0, 1.0, 1.0),
    (1.0023, 0.25, 1.0),
    (2.76, 0.2, 0.45),
    (5.4, 0.035, 0.25),
];

/// One struck tone inside a sound.
#[derive(Clone, Copy, Debug)]
pub struct Strike {
    /// Start time in seconds.
    pub at: f32,
    /// MIDI note number.
    pub note: f32,
    /// Semitones away from the note where the pitch starts. The pitch glides onto the note.
    pub bend: f32,
    /// Decay time constant in seconds.
    pub decay: f32,
    /// Relative loudness.
    pub gain: f32,
}

impl Strike {
    pub const fn new(at: f32, note: f32, decay: f32) -> Self {
        Self {
            at,
            note,
            bend: 0.0,
            decay,
            gain: 1.0,
        }
    }

    pub const fn bend(self, bend: f32) -> Self {
        Self { bend, ..self }
    }

    pub const fn gain(self, gain: f32) -> Self {
        Self { gain, ..self }
    }
}

/// Everything that describes one sound.
#[derive(Clone, Debug)]
pub struct Sound {
    pub partials: Partials,
    pub strikes: Vec<Strike>,
    /// The loudest sample after rendering, as a linear level.
    pub peak: f32,
    /// How much of the room the sound carries, from 0 to 1.
    pub room: f32,
}

/// How long the pitch takes to settle on the note.
const GLIDE: f64 = 0.035;
/// How long a strike takes to reach full level.
const ATTACK: f64 = 0.003;
/// How long the room rings after the last strike fades.
const ROOM_TAIL: f32 = 0.3;
/// The fade at the end of every sound.
const FADE: f32 = 0.02;
/// Samples this far below the loudest one at the end are cut, as a linear ratio (-60 dB).
const SILENCE: f32 = 1e-3;

/// Renders `sound` at the codec rate.
pub fn render(sound: &Sound) -> Vec<f32> {
    let rate = RATE as f32;
    let end = sound
        .strikes
        .iter()
        .map(|s| s.at + s.decay * 6.0)
        .fold(0.0, f32::max)
        + if sound.room > 0.0 { ROOM_TAIL } else { 0.0 };
    let mut out = vec![0.0f32; (end * rate).ceil() as usize];
    for strike in &sound.strikes {
        add_strike(&mut out, strike, sound.partials);
    }
    if sound.room > 0.0 {
        Room::new().apply(&mut out, sound.room);
    }
    finish(&mut out, sound.peak);
    out
}

fn midi_to_hz(note: f64) -> f64 {
    440.0 * 2f64.powf((note - 69.0) / 12.0)
}

fn add_strike(out: &mut [f32], strike: &Strike, partials: Partials) {
    let rate = f64::from(RATE);
    let start = (f64::from(strike.at) * rate) as usize;
    let base = midi_to_hz(f64::from(strike.note));
    for &(ratio, amp, scale) in partials {
        let tau = f64::from(strike.decay * scale);
        let len = ((tau * 8.0 * rate) as usize).min(out.len().saturating_sub(start));
        let mut phase = 0.0f64;
        for i in 0..len {
            let t = i as f64 / rate;
            let bend = f64::from(strike.bend) * (-t / GLIDE).exp();
            let hz = base * 2f64.powf(bend / 12.0) * f64::from(ratio);
            if hz >= rate * 0.45 {
                break;
            }
            phase = (phase + TAU * hz / rate) % TAU;
            let attack = if t < ATTACK {
                0.5 - 0.5 * (std::f64::consts::PI * t / ATTACK).cos()
            } else {
                1.0
            };
            let env = attack * (-t / tau).exp();
            out[start + i] += (f64::from(strike.gain * amp) * env * phase.sin()) as f32;
        }
    }
}

/// Cuts the silent tail, fades the end and scales the loudest sample to `peak`.
fn finish(out: &mut Vec<f32>, peak: f32) {
    let loudest = out.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    let last = out
        .iter()
        .rposition(|s| s.abs() > loudest * SILENCE)
        .unwrap_or(0);
    out.truncate(last + 1);
    let fade = ((FADE * RATE as f32) as usize).min(out.len());
    let len = out.len();
    for (i, s) in out[len - fade..].iter_mut().enumerate() {
        *s *= 1.0 - (i + 1) as f32 / fade as f32;
    }
    if loudest > 0.0 {
        let scale = peak / loudest;
        out.iter_mut().for_each(|s| *s *= scale);
    }
}

/// A feedback comb filter with a damped loop.
struct Comb {
    buf: Vec<f32>,
    i: usize,
    store: f32,
}

impl Comb {
    const FEEDBACK: f32 = 0.72;
    const DAMP: f32 = 0.35;

    fn process(&mut self, x: f32) -> f32 {
        let y = self.buf[self.i];
        self.store = y * (1.0 - Self::DAMP) + self.store * Self::DAMP;
        self.buf[self.i] = x + self.store * Self::FEEDBACK;
        self.i = (self.i + 1) % self.buf.len();
        y
    }
}

/// An allpass diffuser.
struct AllPass {
    buf: Vec<f32>,
    i: usize,
}

impl AllPass {
    const GAIN: f32 = 0.5;

    fn process(&mut self, x: f32) -> f32 {
        let b = self.buf[self.i];
        self.buf[self.i] = x + b * Self::GAIN;
        self.i = (self.i + 1) % self.buf.len();
        b - x
    }
}

/// A small Schroeder room: four combs in parallel, two allpasses in series.
struct Room {
    combs: [Comb; 4],
    passes: [AllPass; 2],
}

impl Room {
    fn new() -> Self {
        let comb = |len| Comb {
            buf: vec![0.0; len],
            i: 0,
            store: 0.0,
        };
        let pass = |len| AllPass {
            buf: vec![0.0; len],
            i: 0,
        };
        Self {
            combs: [comb(1215), comb(1293), comb(1390), comb(1476)],
            passes: [pass(605), pass(480)],
        }
    }

    /// Adds the room to `out` at `wet`.
    fn apply(&mut self, out: &mut [f32], wet: f32) {
        for s in out.iter_mut() {
            let x = *s * 0.25;
            let mut y: f32 = self.combs.iter_mut().map(|c| c.process(x)).sum();
            for p in &mut self.passes {
                y = p.process(y);
            }
            *s += y * wet;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pluck() -> Sound {
        Sound {
            partials: SOFT,
            strikes: vec![Strike::new(0.0, 69.0, 0.1)],
            peak: 0.3,
            room: 0.2,
        }
    }

    #[test]
    fn hits_the_peak() {
        let out = render(&pluck());
        let loudest = out.iter().fold(0.0f32, |m, s| m.max(s.abs()));
        assert!((loudest - 0.3).abs() < 1e-4, "{loudest}");
    }

    #[test]
    fn starts_and_ends_silent() {
        let out = render(&pluck());
        assert!(out[0].abs() < 1e-3);
        assert_eq!(*out.last().unwrap(), 0.0);
    }

    #[test]
    fn glides_onto_the_note() {
        // A bend leaves the end of the strike at the same pitch as an unbent strike.
        let bent = Sound {
            strikes: vec![Strike::new(0.0, 69.0, 0.1).bend(5.0)],
            room: 0.0,
            ..pluck()
        };
        let plain = Sound {
            room: 0.0,
            ..pluck()
        };
        let crossings = |s: &[f32]| s.windows(2).filter(|w| w[0] < 0.0 && w[1] >= 0.0).count();
        let (a, b) = (render(&bent), render(&plain));
        let tail = |s: &[f32]| crossings(&s[9600..19200]);
        assert!(tail(&a).abs_diff(tail(&b)) <= 2);
        assert!(crossings(&a[..2400]) > crossings(&b[..2400]));
    }
}
