//! Capturing a monitor and encoding it to AV1.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use anyhow::{Context as _, Result, anyhow};
use rav1e::prelude::*;

use super::color::rgba_to_i420;
use super::{MonitorInfo, VideoFrame};

/// The widest picture we send. Wider screens are scaled down.
const MAX_WIDTH: usize = 1600;
/// Frames per second we aim for.
pub(super) const FPS: u64 = 15;
/// A keyframe at least this often, so late viewers start within two seconds.
pub(super) const KEY_INTERVAL: u64 = FPS * 2;

/// The monitors xcap can see.
pub fn list_monitors() -> Vec<MonitorInfo> {
    let Ok(monitors) = xcap::Monitor::all() else {
        return Vec::new();
    };
    monitors
        .iter()
        .filter_map(|m| {
            Some(MonitorInfo {
                id: m.id().ok()?,
                name: m
                    .friendly_name()
                    .or_else(|_| m.name())
                    .unwrap_or_else(|_| "Display".into()),
                width: m.width().ok()?,
                height: m.height().ok()?,
                primary: m.is_primary().unwrap_or(false),
            })
        })
        .collect()
}

/// An AV1 encoder tuned for screen content at interactive latency.
pub struct Encoder {
    ctx: Context<u8>,
    headers: super::obu::SequenceHeaderInserter,
    width: usize,
    height: usize,
}

impl Encoder {
    pub fn new(width: usize, height: usize, speed: u8) -> Result<Self> {
        let mut speed_settings = SpeedSettings::from_preset(speed);
        // Interactive latency: no lookahead, no scene-cut analysis, one frame in, one out.
        speed_settings.rdo_lookahead_frames = 1;
        speed_settings.scene_detection_mode = SceneDetectionSpeed::None;
        let enc = EncoderConfig {
            width,
            height,
            time_base: Rational::new(1, FPS),
            low_latency: true,
            min_key_frame_interval: 1,
            max_key_frame_interval: KEY_INTERVAL,
            bitrate: 2_500,
            tiles: 4,
            speed_settings,
            ..Default::default()
        };
        let threads = std::thread::available_parallelism()
            .map(|n| n.get().min(8))
            .unwrap_or(4);
        let cfg = Config::new().with_encoder_config(enc).with_threads(threads);
        let ctx = cfg
            .new_context()
            .map_err(|e| anyhow!("rav1e config: {e}"))?;
        Ok(Self {
            ctx,
            headers: Default::default(),
            width,
            height,
        })
    }

    /// Encodes one RGBA picture of `sw`×`sh`, scaled to the encoder's size.
    pub fn encode(&mut self, rgba: &[u8], sw: usize, sh: usize) -> Result<Vec<VideoFrame>> {
        let pic = rgba_to_i420(rgba, sw, sh, self.width, self.height);
        let mut frame = self.ctx.new_frame();
        frame.planes[0].copy_from_raw_u8(&pic.y, pic.width, 1);
        frame.planes[1].copy_from_raw_u8(&pic.u, pic.width.div_ceil(2), 1);
        frame.planes[2].copy_from_raw_u8(&pic.v, pic.width.div_ceil(2), 1);
        self.ctx
            .send_frame(frame)
            .map_err(|e| anyhow!("rav1e send: {e}"))?;
        let mut out = Vec::new();
        loop {
            match self.ctx.receive_packet() {
                Ok(packet) => {
                    let keyframe = packet.frame_type == FrameType::KEY;
                    out.push(VideoFrame {
                        keyframe,
                        width: self.width as u16,
                        height: self.height as u16,
                        data: self.headers.process(packet.data, keyframe),
                    })
                }
                Err(EncoderStatus::Encoded) => continue,
                Err(EncoderStatus::NeedMoreData) => break,
                Err(e) => return Err(anyhow!("rav1e: {e}")),
            }
        }
        Ok(out)
    }
}

/// The size a `w`×`h` screen is sent at: at most [`MAX_WIDTH`] wide, both sides even.
pub(super) fn target_size(w: usize, h: usize) -> (usize, usize) {
    let scale = (MAX_WIDTH as f32 / w as f32).min(1.0);
    let tw = ((w as f32 * scale) as usize) & !7;
    let th = ((h as f32 * scale) as usize) & !1;
    (tw.max(16), th.max(16))
}

/// A running screen share. Dropping it stops the capture.
pub struct Sharer {
    stop: Arc<AtomicBool>,
    /// Set when the capture ended by itself, such as from the system's sharing controls.
    ended: Arc<AtomicBool>,
}

impl Sharer {
    /// Runs `body` on a capture thread until it returns; `body` watches the stop flag it is given.
    #[cfg_attr(
        not(target_os = "macos"),
        expect(dead_code, reason = "only the macOS capture runs its own thread body")
    )]
    pub(super) fn run(
        ended: Arc<AtomicBool>,
        body: impl FnOnce(&AtomicBool) + Send + 'static,
    ) -> Result<Self> {
        let stop = Arc::new(AtomicBool::new(false));
        let (flag, done) = (stop.clone(), ended.clone());
        std::thread::Builder::new()
            .name("screen-share".into())
            .spawn(move || {
                body(&flag);
                done.store(true, Ordering::Relaxed);
            })
            .context("starting the capture thread")?;
        Ok(Self { stop, ended })
    }

    /// Whether the capture stopped without being asked to.
    pub fn ended(&self) -> bool {
        self.ended.load(Ordering::Relaxed) && !self.stop.load(Ordering::Relaxed)
    }
}

impl Sharer {
    /// Captures monitor `id` and hands encoded frames to `sink`.
    pub fn start(id: u32, sink: impl Fn(VideoFrame) + Send + 'static) -> Result<Self> {
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        // `xcap::Monitor` is not `Send` on every platform, so the monitor is found and opened on
        // the capture thread itself; the first capture's outcome comes back to the caller.
        let (ready_tx, ready_rx) = std::sync::mpsc::channel::<Result<()>>();
        std::thread::Builder::new()
            .name("screen-share".into())
            .spawn(move || {
                let opened = (|| {
                    let monitor = xcap::Monitor::all()
                        .context("listing monitors")?
                        .into_iter()
                        .find(|m| m.id().ok() == Some(id))
                        .ok_or_else(|| anyhow!("that monitor is gone"))?;
                    // Capture once up front so a missing permission is reported to the caller.
                    let first = monitor
                        .capture_image()
                        .map_err(|e| anyhow!("screen capture failed: {e}"))?;
                    let (tw, th) = target_size(first.width() as usize, first.height() as usize);
                    let encoder = Encoder::new(tw, th, 10)?;
                    anyhow::Ok((monitor, encoder))
                })();
                let (monitor, mut encoder) = match opened {
                    Ok(pair) => {
                        let _ = ready_tx.send(Ok(()));
                        pair
                    }
                    Err(e) => {
                        let _ = ready_tx.send(Err(e));
                        return;
                    }
                };
                let period = Duration::from_millis(1000 / FPS);
                let mut next = Instant::now();
                while !flag.load(Ordering::Relaxed) {
                    let image = match monitor.capture_image() {
                        Ok(i) => i,
                        Err(e) => {
                            tracing::warn!("screen capture: {e}");
                            std::thread::sleep(period);
                            continue;
                        }
                    };
                    let (sw, sh) = (image.width() as usize, image.height() as usize);
                    match encoder.encode(image.as_raw(), sw, sh) {
                        Ok(frames) => frames.into_iter().for_each(&sink),
                        Err(e) => tracing::warn!("{e:#}"),
                    }
                    next += period;
                    let now = Instant::now();
                    if next > now {
                        std::thread::sleep(next - now);
                    } else {
                        next = now; // running behind: drop to the encoder's pace
                    }
                }
            })
            .context("starting the capture thread")?;
        ready_rx
            .recv_timeout(Duration::from_secs(10))
            .map_err(|_| anyhow!("the screen did not open in time"))??;
        Ok(Self {
            stop,
            ended: Arc::new(AtomicBool::new(false)),
        })
    }
}

impl Sharer {
    /// Shares a moving test pattern instead of a monitor, for headless testing.
    pub fn test_pattern(sink: impl Fn(VideoFrame) + Send + 'static) -> Result<Self> {
        let (w, h) = (1280usize, 720usize);
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        std::thread::Builder::new()
            .name("test-pattern".into())
            .spawn(move || {
                let Ok(mut encoder) = Encoder::new(w, h, 10) else {
                    return;
                };
                let mut rgba = vec![255u8; w * h * 4];
                let mut t = 0usize;
                while !flag.load(Ordering::Relaxed) {
                    for y in 0..h {
                        for x in 0..w {
                            let i = (y * w + x) * 4;
                            let band = ((x + t * 8) / 80) % 6;
                            let colors = [
                                [229, 72, 77],
                                [247, 107, 21],
                                [255, 197, 61],
                                [70, 167, 88],
                                [0, 144, 255],
                                [142, 78, 198],
                            ];
                            let c = colors[band];
                            let shade = 0.55 + 0.45 * (y as f32 / h as f32);
                            rgba[i] = (c[0] as f32 * shade) as u8;
                            rgba[i + 1] = (c[1] as f32 * shade) as u8;
                            rgba[i + 2] = (c[2] as f32 * shade) as u8;
                        }
                    }
                    if let Ok(frames) = encoder.encode(&rgba, w, h) {
                        frames.into_iter().for_each(&sink);
                    }
                    t += 1;
                    std::thread::sleep(Duration::from_millis(1000 / FPS));
                }
            })?;
        Ok(Self {
            stop,
            ended: Arc::new(AtomicBool::new(false)),
        })
    }
}

impl Drop for Sharer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}
