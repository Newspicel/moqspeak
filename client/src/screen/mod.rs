//! Screen sharing: capture with xcap, AV1 with rav1e, decode with rav1d.
//!
//! A shared screen is the `screen` track of the sharer's broadcast. Every keyframe starts a new
//! MoQ group, so a viewer who subscribes late begins at the latest keyframe.

mod color;
mod decode;
mod encode;
#[cfg(target_os = "macos")]
pub mod mac;
mod obu;

pub use decode::Decoder;
pub use encode::{Sharer, list_monitors};

/// Whether the window offers to share or watch a screen.
pub const ENABLED: bool = false;

/// One encoded video frame and how to place it.
#[derive(Clone, Debug)]
pub struct VideoFrame {
    pub keyframe: bool,
    pub width: u16,
    pub height: u16,
    pub data: Vec<u8>,
}

impl VideoFrame {
    /// Wire format: flags (bit 0 = keyframe), width, height (big-endian u16), AV1 OBUs.
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(5 + self.data.len());
        out.push(self.keyframe as u8);
        out.extend_from_slice(&self.width.to_be_bytes());
        out.extend_from_slice(&self.height.to_be_bytes());
        out.extend_from_slice(&self.data);
        out
    }

    pub fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 5 {
            return None;
        }
        Some(Self {
            keyframe: bytes[0] & 1 == 1,
            width: u16::from_be_bytes([bytes[1], bytes[2]]),
            height: u16::from_be_bytes([bytes[3], bytes[4]]),
            data: bytes[5..].to_vec(),
        })
    }
}

/// Hands a value that is neither `Clone` nor `Debug` through a command that has to be both.
pub struct Handoff<T>(std::sync::Arc<std::sync::Mutex<Option<T>>>);

impl<T> Clone for Handoff<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

#[cfg_attr(
    not(target_os = "macos"),
    expect(dead_code, reason = "only the macOS system picker hands a value over")
)]
impl<T> Handoff<T> {
    pub fn new(value: T) -> Self {
        Self(std::sync::Arc::new(std::sync::Mutex::new(Some(value))))
    }

    pub fn take(&self) -> Option<T> {
        self.0.lock().unwrap().take()
    }
}

impl<T> std::fmt::Debug for Handoff<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Handoff(..)")
    }
}

/// A monitor that can be shared.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct MonitorInfo {
    pub id: u32,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub primary: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn video_frame_round_trips() {
        let f = VideoFrame {
            keyframe: true,
            width: 1280,
            height: 720,
            data: vec![9, 8, 7],
        };
        let g = VideoFrame::decode(&f.encode()).unwrap();
        assert!(g.keyframe);
        assert_eq!((g.width, g.height, g.data), (1280, 720, vec![9, 8, 7]));
    }

    #[test]
    fn av1_round_trips_through_rav1e_and_rav1d() {
        let (w, h) = (1280usize, 720usize);
        let mut rgba = vec![0u8; w * h * 4];
        for y in 0..h {
            for x in 0..w {
                let i = (y * w + x) * 4;
                rgba[i] = (x * 4) as u8;
                rgba[i + 1] = (y * 5) as u8;
                rgba[i + 2] = 128;
                rgba[i + 3] = 255;
            }
        }
        let mut enc = encode::Encoder::new(w, h, 10).unwrap();
        let mut dec = Decoder::new().unwrap();
        let mut decoded = None;
        for _ in 0..6 {
            let frames = enc.encode(&rgba, w, h).unwrap();
            for frame in frames {
                let r = dec.decode(&frame.data).unwrap();
                if let Some(pic) = r {
                    decoded = Some(pic);
                }
            }
        }
        let (dw, dh, out) = decoded.expect("a decoded picture");
        // A viewer that joins at a later keyframe decodes too.
        let mut late = Decoder::new().unwrap();
        for _ in 0..3 {
            enc.encode(&rgba, w, h).unwrap();
        }
        let mut late_picture = None;
        for _ in 0..(super::encode::KEY_INTERVAL as usize + 6) {
            for frame in enc.encode(&rgba, w, h).unwrap() {
                // Like a real late joiner, it is handed delta frames before its first keyframe.
                if let Ok(Some(p)) = late.decode(&frame.data) {
                    late_picture = Some(p);
                }
            }
        }
        assert!(late_picture.is_some(), "late joiner decoded nothing");
        assert_eq!((dw, dh), (w as u32, h as u32));
        // The middle pixel survives within codec tolerance.
        let i = ((h / 2) * w + w / 2) * 4;
        for c in 0..3 {
            assert!(
                (out[i + c] as i32 - rgba[i + c] as i32).abs() < 24,
                "channel {c}"
            );
        }
    }
}

#[cfg(test)]
mod bench {
    /// `cargo test --release -p moqspeak encode_speed -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn encode_speed() {
        let (w, h) = (1600usize, 1000usize);
        let mut rgba = vec![0u8; w * h * 4];
        let mut enc = super::encode::Encoder::new(w, h, 10).unwrap();
        let start = std::time::Instant::now();
        let mut bytes = 0;
        for f in 0..40 {
            for (i, px) in rgba.chunks_mut(4).enumerate() {
                let x = (i % w + f * 7) as u8;
                px.copy_from_slice(&[x, (i / w) as u8, x ^ 0x55, 255]);
            }
            bytes += enc
                .encode(&rgba, w, h)
                .unwrap()
                .iter()
                .map(|p| p.data.len())
                .sum::<usize>();
        }
        let secs = start.elapsed().as_secs_f64();
        eprintln!("{:.1} fps, {} kB total", 40.0 / secs, bytes / 1000);
    }
}

#[cfg(test)]
mod live_path {
    use super::*;

    /// The test-pattern sharer, through the wire format, into a decoder that joins late.
    #[test]
    fn test_pattern_decodes_after_late_join() {
        let (tx, rx) = std::sync::mpsc::channel();
        let sharer = Sharer::test_pattern(move |f| {
            let _ = tx.send(f.encode());
        })
        .unwrap();
        // Skip the first second so the decoder starts mid-stream.
        std::thread::sleep(std::time::Duration::from_millis(1000));
        while rx.try_recv().is_ok() {}
        let mut decoder = Decoder::new().unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(8);
        let mut got = None;
        let (mut frames, mut keys) = (0, 0);
        while std::time::Instant::now() < deadline && got.is_none() {
            let Ok(bytes) = rx.recv_timeout(std::time::Duration::from_millis(500)) else {
                continue;
            };
            let frame = VideoFrame::decode(&bytes).unwrap();
            frames += 1;
            keys += frame.keyframe as u32;
            match decoder.decode(&frame.data) {
                Ok(Some(p)) => got = Some((p.0, p.1)),
                Ok(None) => {}
                Err(e) => eprintln!("decode error: {e:#}"),
            }
        }
        drop(sharer);
        assert_eq!(got, Some((1280, 720)), "frames={frames} keys={keys}");
    }
}
