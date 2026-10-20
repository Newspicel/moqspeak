//! One remote speaker's jitter buffer. Frames arrive in bursts and play out on the sound card's
//! clock, so the buffer holds what has arrived and sheds the delay it carries above the
//! prebuffer.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use anyhow::Result;

use super::{FRAME, Packet, RATE};

/// How much audio a speaker buffers before playback starts, and the depth it settles back to.
const PREBUFFER: usize = FRAME * 3;
/// The most audio a speaker may hold before the oldest is dropped.
const MAX_BUFFER: usize = FRAME * 20;
/// How much audio plays between two readings of the depth the buffer keeps.
const TRIM_PERIOD: usize = RATE as usize * 2;
/// How long a trim blends its splice over.
const TRIM_FADE: usize = FRAME / 4;

pub(super) struct Peer {
    decoder: opus::Decoder,
    queue: VecDeque<f32>,
    next_seq: Option<u32>,
    playing: bool,
    pub(super) volume: f32,
    pub(super) muted: bool,
    last_packet: Instant,
    /// The shallowest the queue has been since the last trim.
    low_water: usize,
    /// Samples played since the last trim.
    played: usize,
}

impl Peer {
    pub(super) fn new() -> Result<Self> {
        Ok(Self {
            decoder: opus::Decoder::new(RATE, opus::Channels::Mono)?,
            queue: VecDeque::with_capacity(MAX_BUFFER),
            next_seq: None,
            playing: false,
            volume: 1.0,
            muted: false,
            last_packet: Instant::now() - Duration::from_secs(10),
            low_water: usize::MAX,
            played: 0,
        })
    }

    pub(super) fn push(&mut self, packet: &Packet) {
        let mut pcm = [0f32; FRAME * 6];
        // Conceal a short gap with the decoder's loss concealment; a long gap is a new spurt.
        if let Some(expected) = self.next_seq {
            let gap = packet.seq.wrapping_sub(expected);
            if gap > 0 && gap <= 3 {
                for _ in 0..gap {
                    if let Ok(n) = self.decoder.decode_float(&[], &mut pcm[..FRAME], false) {
                        self.queue.extend(&pcm[..n]);
                    }
                }
            } else if gap > 3 && gap < u32::MAX / 2 {
                self.playing = false;
            } else if gap >= u32::MAX / 2 {
                return; // late duplicate
            }
        }
        self.next_seq = Some(packet.seq.wrapping_add(1));
        if let Ok(n) = self.decoder.decode_float(&packet.data, &mut pcm, false) {
            self.queue.extend(&pcm[..n]);
        }
        while self.queue.len() > MAX_BUFFER {
            self.queue.pop_front();
        }
        if !self.playing && self.queue.len() >= PREBUFFER {
            self.playing = true;
        }
        self.last_packet = Instant::now();
    }

    pub(super) fn pull(&mut self) -> f32 {
        if !self.playing {
            return 0.0;
        }
        match self.queue.pop_front() {
            Some(s) => {
                self.watch_depth();
                s
            }
            None => {
                self.playing = false;
                0.0
            }
        }
    }

    /// What this speaker contributes to the mix.
    pub(super) fn gain(&self) -> f32 {
        if self.muted { 0.0 } else { self.volume }
    }

    /// Whether a frame arrived within `window`.
    pub(super) fn heard_within(&self, window: Duration) -> bool {
        self.last_packet.elapsed() < window
    }

    /// Follows the depth the queue keeps and sheds the delay above the prebuffer. The shallowest
    /// reading over a period is the delay that period never needed.
    fn watch_depth(&mut self) {
        self.low_water = self.low_water.min(self.queue.len());
        self.played += 1;
        if self.played < TRIM_PERIOD {
            return;
        }
        let excess = self.low_water.saturating_sub(PREBUFFER);
        self.played = 0;
        self.low_water = self.queue.len();
        if excess > 0 {
            self.trim(excess);
        }
    }

    /// Drops `count` samples from the front, and blends over the splice.
    fn trim(&mut self, count: usize) {
        let count = count.min(self.queue.len());
        let fade = TRIM_FADE.min(count).min(self.queue.len() - count);
        let blend: Vec<f32> = (0..fade)
            .map(|i| {
                let weight = i as f32 / fade as f32;
                self.queue[i] * (1.0 - weight) + self.queue[count + i] * weight
            })
            .collect();
        self.queue.drain(..count);
        for (slot, sample) in self.queue.iter_mut().zip(blend) {
            *slot = sample;
        }
        self.low_water = self.queue.len();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A speaker fed `frames` at a time, every `frames` frames of playback.
    struct Stream {
        encoder: opus::Encoder,
        buf: [u8; 1500],
        seq: u32,
        phase: f32,
    }

    impl Stream {
        fn new() -> Self {
            Self {
                encoder: opus::Encoder::new(RATE, opus::Channels::Mono, opus::Application::Audio)
                    .unwrap(),
                buf: [0u8; 1500],
                seq: 0,
                phase: 0.0,
            }
        }

        fn frame(&mut self) -> Packet {
            let pcm: Vec<f32> = (0..FRAME)
                .map(|_| {
                    self.phase = (self.phase + 440.0 * std::f32::consts::TAU / RATE as f32)
                        % std::f32::consts::TAU;
                    self.phase.sin() * 0.5
                })
                .collect();
            let n = self.encoder.encode_float(&pcm, &mut self.buf).unwrap();
            let packet = Packet {
                seq: self.seq,
                data: self.buf[..n].to_vec(),
            };
            self.seq = self.seq.wrapping_add(1);
            packet
        }
    }

    /// Plays `seconds` of audio. `backlog` frames wait at the start, as they do on joining a
    /// channel that is already streaming, and `burst` frames arrive together from then on.
    /// Reports the shallowest depth of the last second, and whether the buffer ever ran dry.
    fn run(
        peer: &mut Peer,
        stream: &mut Stream,
        seconds: f32,
        burst: usize,
        backlog: usize,
    ) -> (usize, bool) {
        for _ in 0..backlog {
            peer.push(&stream.frame());
        }
        let total = (RATE as f32 * seconds) as usize;
        let last_second = total.saturating_sub(RATE as usize);
        let mut low = usize::MAX;
        let mut starved = false;
        let mut owed = 0usize;
        for i in 0..total {
            if owed == 0 {
                for _ in 0..burst {
                    peer.push(&stream.frame());
                }
                owed = FRAME * burst;
            }
            owed -= 1;
            peer.pull();
            starved |= peer.queue.is_empty();
            if i >= last_second {
                low = low.min(peer.queue.len());
            }
        }
        (low, starved)
    }

    /// A TeamSpeak music channel sends six frames at a time, and what it has buffered already
    /// arrives at once on joining. That head start is delay, and it has to drain away.
    #[test]
    fn sheds_the_delay_a_burst_of_frames_leaves_behind() {
        let (mut peer, mut stream) = (Peer::new().unwrap(), Stream::new());
        let (low, starved) = run(&mut peer, &mut stream, 10.0, 6, 12);
        assert!(!starved);
        assert!(
            low <= PREBUFFER + FRAME,
            "the buffer still stands {} ms deep",
            low * 1000 / RATE as usize
        );
    }

    #[test]
    fn keeps_the_prebuffer_in_hand() {
        let (mut peer, mut stream) = (Peer::new().unwrap(), Stream::new());
        let (low, _) = run(&mut peer, &mut stream, 10.0, 6, 12);
        assert!(low >= FRAME, "only {low} samples left to play from");
    }

    #[test]
    fn a_steady_stream_needs_no_trim() {
        let (mut peer, mut stream) = (Peer::new().unwrap(), Stream::new());
        let (low, starved) = run(&mut peer, &mut stream, 10.0, 1, 3);
        assert!(!starved);
        assert!(low <= PREBUFFER + FRAME);
        assert!(low >= FRAME);
    }

    #[test]
    fn a_trim_leaves_no_step_in_the_signal() {
        let mut peer = Peer::new().unwrap();
        // Two cycles of a tone, a quarter cycle apart, so a raw splice would step.
        let tone = |i: usize| (i as f32 * std::f32::consts::TAU / 480.0).sin();
        peer.queue.extend((0..FRAME * 8).map(tone));
        let steepest = |q: &VecDeque<f32>| {
            q.iter()
                .zip(q.iter().skip(1))
                .map(|(a, b)| (b - a).abs())
                .fold(0.0f32, f32::max)
        };
        let before = steepest(&peer.queue);
        peer.trim(FRAME + 120);
        let after = steepest(&peer.queue);
        assert!(
            after <= before * 1.5,
            "the splice steps by {after}, the tone by {before}"
        );
    }
}
