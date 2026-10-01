//! Microphone capture, Opus encoding, and mixed playback of the remote speakers.
//!
//! The codec runs at 48 kHz mono with 20 ms frames. Device streams run at whatever rate the
//! device prefers; a small linear resampler sits on each side.

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::Result;

/// Codec sample rate.
pub const RATE: u32 = 48_000;
/// Samples in one 20 ms frame.
pub const FRAME: usize = 960;
/// The peer key the microphone loopback test plays under.
pub const LOOPBACK_PEER: u64 = 0;

/// How the microphone decides when to send.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum VoiceMode {
    /// Send while the level is above the threshold.
    Activation,
    /// Send while the push-to-talk key is held.
    PushToTalk,
    /// Send all the time.
    Continuous,
}

impl VoiceMode {
    fn to_u8(self) -> u8 {
        match self {
            VoiceMode::Activation => 0,
            VoiceMode::PushToTalk => 1,
            VoiceMode::Continuous => 2,
        }
    }
    fn from_u8(v: u8) -> Self {
        match v {
            1 => VoiceMode::PushToTalk,
            2 => VoiceMode::Continuous,
            _ => VoiceMode::Activation,
        }
    }
}

/// An `f32` that threads read and write without a lock.
#[derive(Default)]
pub struct AtomicF32(AtomicU32);

impl AtomicF32 {
    pub fn new(v: f32) -> Self {
        Self(AtomicU32::new(v.to_bits()))
    }
    pub fn get(&self) -> f32 {
        f32::from_bits(self.0.load(Ordering::Relaxed))
    }
    pub fn set(&self, v: f32) {
        self.0.store(v.to_bits(), Ordering::Relaxed)
    }
}

/// Settings and meters shared between the UI, the network and the audio threads.
pub struct AudioShared {
    mode: AtomicU8,
    /// Voice activation threshold in dBFS.
    pub threshold_db: AtomicF32,
    pub ptt_down: AtomicBool,
    pub mic_muted: AtomicBool,
    pub deafened: AtomicBool,
    pub loopback: AtomicBool,
    /// Linear microphone gain.
    pub input_gain: AtomicF32,
    /// Linear output volume.
    pub master_volume: AtomicF32,
    /// Current microphone level in dBFS, after gain.
    pub level_db: AtomicF32,
    /// Whether the encoder is sending right now.
    pub transmitting: AtomicBool,
}

impl AudioShared {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            mode: AtomicU8::new(VoiceMode::Activation.to_u8()),
            threshold_db: AtomicF32::new(-38.0),
            ptt_down: AtomicBool::new(false),
            mic_muted: AtomicBool::new(false),
            deafened: AtomicBool::new(false),
            loopback: AtomicBool::new(false),
            input_gain: AtomicF32::new(1.0),
            master_volume: AtomicF32::new(1.0),
            level_db: AtomicF32::new(-100.0),
            transmitting: AtomicBool::new(false),
        })
    }
    pub fn mode(&self) -> VoiceMode {
        VoiceMode::from_u8(self.mode.load(Ordering::Relaxed))
    }
    pub fn set_mode(&self, mode: VoiceMode) {
        self.mode.store(mode.to_u8(), Ordering::Relaxed)
    }
}

/// One encoded 20 ms frame and its position in the talk spurt.
#[derive(Clone, Debug)]
pub struct Packet {
    pub seq: u32,
    pub data: Vec<u8>,
}

impl Packet {
    /// Wire format: 4-byte big-endian sequence number, then the Opus payload.
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(4 + self.data.len());
        out.extend_from_slice(&self.seq.to_be_bytes());
        out.extend_from_slice(&self.data);
        out
    }
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 4 {
            return None;
        }
        let seq = u32::from_be_bytes(bytes[..4].try_into().ok()?);
        Some(Self {
            seq,
            data: bytes[4..].to_vec(),
        })
    }
}

// ---- playback mixer ------------------------------------------------------------------------

/// How much audio a speaker buffers before playback starts.
const PREBUFFER: usize = FRAME * 3;
/// The most audio a speaker may hold before the oldest is dropped.
const MAX_BUFFER: usize = FRAME * 12;

struct Peer {
    decoder: opus::Decoder,
    queue: VecDeque<f32>,
    next_seq: Option<u32>,
    playing: bool,
    volume: f32,
    muted: bool,
    last_packet: Instant,
}

impl Peer {
    fn new() -> Result<Self> {
        Ok(Self {
            decoder: opus::Decoder::new(RATE, opus::Channels::Mono)?,
            queue: VecDeque::with_capacity(MAX_BUFFER),
            next_seq: None,
            playing: false,
            volume: 1.0,
            muted: false,
            last_packet: Instant::now() - Duration::from_secs(10),
        })
    }

    fn push(&mut self, packet: &Packet) {
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

    fn pull(&mut self) -> f32 {
        if !self.playing {
            return 0.0;
        }
        match self.queue.pop_front() {
            Some(s) => s,
            None => {
                self.playing = false;
                0.0
            }
        }
    }
}

/// The remote speakers, keyed by client id.
#[derive(Default)]
pub struct Mixer {
    peers: Mutex<HashMap<u64, Peer>>,
}

impl Mixer {
    pub fn push(&self, peer: u64, packet: &Packet) {
        let mut peers = self.peers.lock().unwrap();
        let entry = match peers.entry(peer) {
            std::collections::hash_map::Entry::Occupied(e) => e.into_mut(),
            std::collections::hash_map::Entry::Vacant(v) => match Peer::new() {
                Ok(p) => v.insert(p),
                Err(_) => return,
            },
        };
        entry.push(packet);
    }

    pub fn remove(&self, peer: u64) {
        self.peers.lock().unwrap().remove(&peer);
    }

    pub fn set_volume(&self, peer: u64, volume: f32) {
        let mut peers = self.peers.lock().unwrap();
        if let Some(p) = peers.get_mut(&peer) {
            p.volume = volume;
        } else if let Ok(mut p) = Peer::new() {
            p.volume = volume;
            peers.insert(peer, p);
        }
    }

    pub fn set_muted(&self, peer: u64, muted: bool) {
        let mut peers = self.peers.lock().unwrap();
        if let Some(p) = peers.get_mut(&peer) {
            p.muted = muted;
        } else if let Ok(mut p) = Peer::new() {
            p.muted = muted;
            peers.insert(peer, p);
        }
    }

    /// The peers that received audio within `window`.
    pub fn talking(&self, window: Duration) -> Vec<u64> {
        let now = Instant::now();
        self.peers
            .lock()
            .unwrap()
            .iter()
            .filter(|(_, p)| now.duration_since(p.last_packet) < window)
            .map(|(id, _)| *id)
            .collect()
    }

    /// Mixes `out.len()` samples at the codec rate.
    fn mix(&self, out: &mut [f32]) {
        out.fill(0.0);
        let mut peers = self.peers.lock().unwrap();
        for peer in peers.values_mut() {
            let gain = if peer.muted { 0.0 } else { peer.volume };
            for s in out.iter_mut() {
                *s += peer.pull() * gain;
            }
        }
    }
}

// ---- resampling ----------------------------------------------------------------------------

/// A streaming linear resampler.
struct Resampler {
    ratio: f64, // input samples per output sample
    pos: f64,
    prev: f32,
}

impl Resampler {
    fn new(from: u32, to: u32) -> Self {
        Self {
            ratio: from as f64 / to as f64,
            pos: 0.0,
            prev: 0.0,
        }
    }

    /// Converts `input` and appends the result to `out`.
    fn process(&mut self, input: &[f32], out: &mut Vec<f32>) {
        if (self.ratio - 1.0).abs() < 1e-9 {
            out.extend_from_slice(input);
            return;
        }
        // `pos` is measured from `prev`, which sits one sample before `input[0]`.
        let len = input.len() as f64;
        while self.pos < len {
            let i = self.pos.floor() as usize;
            let frac = (self.pos - i as f64) as f32;
            let a = if i == 0 { self.prev } else { input[i - 1] };
            let b = input[i.min(input.len() - 1)];
            out.push(a + (b - a) * frac);
            self.pos += self.ratio;
        }
        self.pos -= len;
        if let Some(&last) = input.last() {
            self.prev = last;
        }
    }
}

mod devices;

pub use devices::{DeviceInfo, list_devices};

/// Owns the device streams on a thread of their own and the encoder on another.
pub struct Audio {
    pub shared: Arc<AudioShared>,
    pub mixer: Arc<Mixer>,
    devices: Option<devices::DeviceThread>,
}

impl Audio {
    /// Shared state and a mixer with no devices behind them, for the headless bot.
    pub fn headless() -> Self {
        Self { shared: AudioShared::new(), mixer: Arc::new(Mixer::default()), devices: None }
    }

    /// Opens the named devices, or the system defaults for `None`. Encoded packets go to
    /// `packets`.
    pub fn start(
        packets: tokio::sync::mpsc::UnboundedSender<Packet>,
        input: Option<String>,
        output: Option<String>,
    ) -> Self {
        let shared = AudioShared::new();
        let mixer = Arc::new(Mixer::default());
        let (frame_tx, frame_rx) = std::sync::mpsc::channel::<Vec<f32>>();
        let devices = devices::DeviceThread::spawn(shared.clone(), mixer.clone(), frame_tx, input, output);
        {
            let shared = shared.clone();
            let mixer = mixer.clone();
            std::thread::Builder::new()
                .name("audio-encoder".into())
                .spawn(move || encoder_loop(shared, mixer, frame_rx, packets))
                .expect("spawn encoder thread");
        }
        Self { shared, mixer, devices: Some(devices) }
    }

    /// What is open right now.
    pub fn device_info(&self) -> DeviceInfo {
        self.devices.as_ref().map(|d| d.info()).unwrap_or_default()
    }

    /// Reopens the microphone; `None` follows the system default.
    pub fn set_input(&self, name: Option<String>) -> DeviceInfo {
        self.devices.as_ref().map(|d| d.set_input(name)).unwrap_or_default()
    }

    /// Reopens the speakers; `None` follows the system default.
    pub fn set_output(&self, name: Option<String>) -> DeviceInfo {
        self.devices.as_ref().map(|d| d.set_output(name)).unwrap_or_default()
    }
}

fn level_db(frame: &[f32]) -> f32 {
    let energy = frame.iter().map(|s| s * s).sum::<f32>() / frame.len().max(1) as f32;
    10.0 * energy.max(1e-10).log10()
}

fn encoder_loop(
    shared: Arc<AudioShared>,
    mixer: Arc<Mixer>,
    frames: std::sync::mpsc::Receiver<Vec<f32>>,
    packets: tokio::sync::mpsc::UnboundedSender<Packet>,
) {
    let mut encoder = match opus::Encoder::new(RATE, opus::Channels::Mono, opus::Application::Voip)
    {
        Ok(e) => e,
        Err(e) => {
            tracing::error!("opus encoder: {e}");
            return;
        }
    };
    let _ = encoder.set_bitrate(opus::Bitrate::Bits(40_000));
    let _ = encoder.set_inband_fec(true);
    let _ = encoder.set_packet_loss_perc(5);

    let mut buffer: Vec<f32> = Vec::with_capacity(FRAME * 4);
    let mut out = [0u8; 1500];
    let mut seq: u32 = 0;
    let mut hangover = 0u32; // frames left before activation closes
    const HANGOVER_FRAMES: u32 = 20; // 400 ms
    let mut smoothed = -100.0f32;

    while let Ok(chunk) = frames.recv() {
        buffer.extend_from_slice(&chunk);
        while buffer.len() >= FRAME {
            let frame: Vec<f32> = buffer.drain(..FRAME).collect();
            let level = level_db(&frame);
            smoothed = if level > smoothed {
                level
            } else {
                smoothed * 0.8 + level * 0.2
            };
            shared.level_db.set(smoothed);

            let open = match shared.mode() {
                VoiceMode::Continuous => true,
                VoiceMode::PushToTalk => shared.ptt_down.load(Ordering::Relaxed),
                VoiceMode::Activation => {
                    if level >= shared.threshold_db.get() {
                        hangover = HANGOVER_FRAMES;
                    } else {
                        hangover = hangover.saturating_sub(1);
                    }
                    hangover > 0
                }
            };
            let sending = open && !shared.mic_muted.load(Ordering::Relaxed);
            let was = shared.transmitting.swap(sending, Ordering::Relaxed);
            if !sending {
                if was {
                    // Leave a gap in the sequence so receivers treat the next spurt as new.
                    seq = seq.wrapping_add(16);
                    let _ = encoder.reset_state();
                }
                continue;
            }
            match encoder.encode_float(&frame, &mut out) {
                Ok(n) => {
                    let packet = Packet {
                        seq,
                        data: out[..n].to_vec(),
                    };
                    seq = seq.wrapping_add(1);
                    if shared.loopback.load(Ordering::Relaxed) {
                        mixer.push(LOOPBACK_PEER, &packet);
                    }
                    let _ = packets.send(packet);
                }
                Err(e) => tracing::warn!("opus encode: {e}"),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packet_round_trips() {
        let p = Packet {
            seq: 0xdead_beef,
            data: vec![1, 2, 3],
        };
        let q = Packet::decode(&p.encode()).unwrap();
        assert_eq!(q.seq, p.seq);
        assert_eq!(q.data, p.data);
    }

    #[test]
    fn resampler_keeps_rate() {
        let mut r = Resampler::new(44_100, 48_000);
        let mut out = Vec::new();
        for _ in 0..100 {
            r.process(&[0.5; 441], &mut out);
        }
        assert!((out.len() as i64 - 48_000).abs() < 4, "{}", out.len());
    }

    #[test]
    fn mixer_plays_after_prebuffer() {
        let mut enc =
            opus::Encoder::new(RATE, opus::Channels::Mono, opus::Application::Voip).unwrap();
        let mixer = Mixer::default();
        let tone: Vec<f32> = (0..FRAME).map(|i| (i as f32 * 0.05).sin() * 0.5).collect();
        let mut buf = [0u8; 1500];
        for seq in 0..4 {
            let n = enc.encode_float(&tone, &mut buf).unwrap();
            mixer.push(
                7,
                &Packet {
                    seq,
                    data: buf[..n].to_vec(),
                },
            );
        }
        let mut out = vec![0.0; FRAME];
        mixer.mix(&mut out);
        assert!(out.iter().any(|s| s.abs() > 0.01));
        assert_eq!(mixer.talking(Duration::from_secs(1)), vec![7]);
    }
}
