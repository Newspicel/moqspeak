//! Cleaning and classifying the microphone: WebRTC AEC3 echo cancellation (sonora), RNNoise
//! suppression and the earshot speech detector.

use nnnoiseless::DenoiseState;

use sonora::config::{EchoCanceller as Aec3, HighPassFilter};
use sonora::{AudioProcessing, Config, StreamConfig};

use super::{FRAME, RATE};

/// Samples in the 10 ms frames sonora works on.
const TEN_MS: usize = RATE as usize / 100;

/// Removes what the speakers played from the microphone signal.
pub struct EchoCanceller {
    apm: AudioProcessing,
    scratch: Vec<f32>,
}

impl EchoCanceller {
    pub fn new() -> Self {
        let config = Config {
            echo_canceller: Some(Aec3::default()),
            high_pass_filter: Some(HighPassFilter::default()),
            ..Default::default()
        };
        let mut apm = AudioProcessing::builder()
            .config(config)
            .capture_config(StreamConfig::new(RATE, 1))
            .render_config(StreamConfig::new(RATE, 1))
            .build();
        // A first guess for device latency; AEC3 refines its own estimate.
        let _ = apm.set_stream_delay_ms(50);
        Self {
            apm,
            scratch: vec![0.0; TEN_MS],
        }
    }

    /// Feeds the far end (`played`, consumed in whole 10 ms frames) and cleans `frame` in place.
    pub fn process(&mut self, frame: &mut [f32], played: &mut Vec<f32>) {
        let whole = played.len() / TEN_MS * TEN_MS;
        for chunk in played[..whole].chunks_exact(TEN_MS) {
            let _ = self
                .apm
                .process_render_f32(&[chunk], &mut [&mut self.scratch[..]]);
        }
        played.drain(..whole);
        for chunk in frame.chunks_exact_mut(TEN_MS) {
            self.scratch.copy_from_slice(chunk);
            let _ = self
                .apm
                .process_capture_f32(&[&self.scratch[..]], &mut [chunk]);
        }
    }
}

/// The speech probability above which a frame counts as voice.
pub const SPEECH: f32 = 0.5;

/// RNNoise at 48 kHz, in its own 480-sample frames.
pub struct Cleaner {
    state: Box<DenoiseState<'static>>,
    scratch_in: Vec<f32>,
    scratch_out: Vec<f32>,
}

impl Cleaner {
    pub fn new() -> Self {
        Self {
            state: DenoiseState::new(),
            scratch_in: vec![0.0; DenoiseState::FRAME_SIZE],
            scratch_out: vec![0.0; DenoiseState::FRAME_SIZE],
        }
    }

    /// Denoises a codec frame in place. RNNoise works on 16-bit scaled samples.
    pub fn process(&mut self, frame: &mut [f32]) {
        const N: usize = DenoiseState::FRAME_SIZE;
        debug_assert_eq!(FRAME % N, 0);
        for chunk in frame.chunks_mut(N) {
            for (dst, src) in self.scratch_in.iter_mut().zip(chunk.iter()) {
                *dst = src * 32768.0;
            }
            self.state
                .process_frame(&mut self.scratch_out, &self.scratch_in);
            for (dst, src) in chunk.iter_mut().zip(self.scratch_out.iter()) {
                *dst = src / 32768.0;
            }
        }
    }
}

/// earshot wants 256-sample frames at 16 kHz; the codec runs at 48 kHz.
pub struct SpeechDetector {
    detector: earshot::Detector,
    pending: Vec<f32>,
    last: f32,
}

impl SpeechDetector {
    pub fn new() -> Self {
        Self {
            detector: earshot::Detector::default(),
            pending: Vec::with_capacity(512),
            last: 0.0,
        }
    }

    /// Feeds a 48 kHz frame and returns the latest speech probability.
    pub fn push(&mut self, frame: &[f32]) -> f32 {
        // Decimate by three with a box filter: crude, and plenty for a detector.
        for tri in frame.chunks_exact(3) {
            self.pending.push((tri[0] + tri[1] + tri[2]) / 3.0);
        }
        while self.pending.len() >= 256 {
            let chunk: Vec<f32> = self.pending.drain(..256).collect();
            self.last = self.detector.predict_f32(&chunk);
        }
        self.last
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn echo_canceller_removes_a_delayed_copy_of_the_far_end() {
        let mut aec = EchoCanceller::new();
        let mut seed = 0x1234_5678u32;
        let mut noise = move || {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            (seed as f32 / u32::MAX as f32 - 0.5) * 0.4
        };
        let delay = RATE as usize / 50; // 20 ms between the speaker and the microphone
        let mut history = vec![0.0f32; delay];
        let (mut before, mut after) = (0.0f32, 0.0f32);
        for i in 0..250 {
            // 5 s of 20 ms frames
            let far: Vec<f32> = (0..FRAME).map(|_| noise()).collect();
            history.extend_from_slice(&far);
            let mut mic: Vec<f32> = history[..FRAME].iter().map(|s| s * 0.5).collect();
            history.drain(..FRAME);
            let mut played = far.clone();
            let energy_in: f32 = mic.iter().map(|s| s * s).sum();
            aec.process(&mut mic, &mut played);
            if i >= 200 {
                before += energy_in;
                after += mic.iter().map(|s| s * s).sum::<f32>();
            }
        }
        let reduction_db = 10.0 * (before / after.max(1e-12)).log10();
        assert!(
            reduction_db > 10.0,
            "only {reduction_db:.1} dB of echo removed"
        );
    }

    #[test]
    fn silence_is_not_speech() {
        let mut d = SpeechDetector::new();
        let mut p = 1.0;
        for _ in 0..20 {
            p = d.push(&[0.0; FRAME]);
        }
        assert!(p < SPEECH, "{p}");
    }

    #[test]
    fn cleaner_keeps_length_and_silence() {
        let mut c = Cleaner::new();
        let mut frame = vec![0.0; FRAME];
        c.process(&mut frame);
        assert_eq!(frame.len(), FRAME);
        assert!(frame.iter().all(|s| s.abs() < 1e-3));
    }
}
