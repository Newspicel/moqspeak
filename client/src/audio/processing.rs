//! Cleaning and classifying the microphone: RNNoise suppression and the earshot speech detector.

use nnnoiseless::DenoiseState;

use super::FRAME;

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
