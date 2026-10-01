//! macOS: Apple's content-sharing picker (window, app or display) and ScreenCaptureKit.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{Result, anyhow};
use screencapturekit::cm::{CMSampleBufferExt, CMTime};
use screencapturekit::content_sharing_picker::{
    SCContentSharingPicker, SCContentSharingPickerConfiguration, SCContentSharingPickerMode,
    SCPickerOutcome,
};
use screencapturekit::cv::CVPixelBufferLockFlags;
use screencapturekit::prelude::*;

use super::VideoFrame;
use super::encode::{Encoder, FPS, Sharer, target_size};

/// What the user picked, ready to be captured.
pub struct Picked {
    filter: SCContentFilter,
    size: (u32, u32),
}

impl std::fmt::Debug for Picked {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Picked({}x{})", self.size.0, self.size.1)
    }
}

/// Whether the system picker exists here (macOS 14 and later).
pub fn picker_available() -> bool {
    SCContentSharingPicker::is_available()
}

/// Shows the system picker. `done` receives the choice, `None` when the user cancelled.
pub fn pick(done: impl FnOnce(Result<Option<Picked>>) + Send + 'static) {
    let mut config = match SCContentSharingPickerConfiguration::new() {
        Ok(c) => c,
        Err(e) => return done(Err(anyhow!("the system picker is unavailable: {e}"))),
    };
    config.set_allowed_picker_modes(&[
        SCContentSharingPickerMode::SingleWindow,
        SCContentSharingPickerMode::SingleApplication,
        SCContentSharingPickerMode::SingleDisplay,
    ]);
    tracing::debug!("showing the system content picker");
    SCContentSharingPicker::show(&config, move |outcome| match outcome {
        SCPickerOutcome::Picked(result) => done(Ok(Some(Picked {
            filter: result.filter(),
            size: result.pixel_size(),
        }))),
        SCPickerOutcome::Cancelled => {
            tracing::debug!("content picker cancelled");
            done(Ok(None))
        }
        SCPickerOutcome::Error(e) => done(Err(anyhow!("screen picker: {e}"))),
    });
}

/// One captured picture, RGBA, tightly packed.
struct Frame {
    rgba: Vec<u8>,
    width: usize,
    height: usize,
}

/// Copies each new frame into the slot the encoder reads from.
struct Output {
    latest: Arc<Mutex<Option<Frame>>>,
}

impl SCStreamOutputTrait for Output {
    fn did_output_sample_buffer(&self, sample: CMSampleBuffer, kind: SCStreamOutputType) {
        if !matches!(kind, SCStreamOutputType::Screen) {
            return;
        }
        let Some(pixels) = sample.pixel_buffer() else {
            return;
        };
        let Ok(guard) = pixels.lock(CVPixelBufferLockFlags::READ_ONLY) else {
            return;
        };
        let (width, height, stride) = (guard.width(), guard.height(), guard.bytes_per_row());
        // SAFETY: the buffer stays locked for as long as `guard` lives.
        let Some(bytes) = (unsafe { guard.as_slice() }) else {
            return;
        };
        if width == 0 || height == 0 || bytes.len() < stride * height {
            return;
        }
        let mut rgba = Vec::with_capacity(width * height * 4);
        for row in bytes.chunks(stride).take(height) {
            for px in row[..width * 4].chunks_exact(4) {
                rgba.extend_from_slice(&[px[2], px[1], px[0], 255]);
            }
        }
        *self.latest.lock().unwrap() = Some(Frame {
            rgba,
            width,
            height,
        });
    }
}

/// Notices when the stream stops on its own, such as from the menu bar's sharing control.
struct Watchdog {
    ended: Arc<AtomicBool>,
}

impl SCStreamDelegateTrait for Watchdog {
    fn did_stop_with_error(&self, error: SCError) {
        tracing::info!("screen capture stopped: {error}");
        self.ended.store(true, Ordering::Relaxed);
    }

    fn stream_did_become_inactive(&self) {
        self.ended.store(true, Ordering::Relaxed);
    }
}

/// Captures `picked` and hands encoded frames to `sink`.
pub fn start(picked: Picked, sink: impl Fn(VideoFrame) + Send + 'static) -> Result<Sharer> {
    let (tw, th) = target_size(
        picked.size.0.max(16) as usize,
        picked.size.1.max(16) as usize,
    );
    let config = SCStreamConfiguration::new()
        .with_width(tw as u32)
        .with_height(th as u32)
        .with_scales_to_fit(true)
        .with_shows_cursor(true)
        .with_queue_depth(3)
        .with_pixel_format(PixelFormat::BGRA)
        .with_minimum_frame_interval(&CMTime::new(1, FPS as i32));

    let latest = Arc::new(Mutex::new(None::<Frame>));
    let ended = Arc::new(AtomicBool::new(false));
    let mut stream = SCStream::new_with_delegate(
        &picked.filter,
        &config,
        Watchdog {
            ended: ended.clone(),
        },
    )
    .map_err(|e| anyhow!("screen capture: {e}"))?;
    stream
        .add_output_handler(
            Output {
                latest: latest.clone(),
            },
            SCStreamOutputType::Screen,
        )
        .map_err(|e| anyhow!("screen capture: {e}"))?;
    stream
        .start_capture()
        .map_err(|e| anyhow!("screen capture failed to start: {e}"))?;

    let mut encoder = Encoder::new(tw, th, 10)?;
    Sharer::run(ended, move |stop: &AtomicBool| {
        let period = Duration::from_millis(1000 / FPS);
        let mut last: Option<Frame> = None;
        let mut last_encoded = Instant::now() - Duration::from_secs(1);
        while !stop.load(Ordering::Relaxed) {
            let started = Instant::now();
            let fresh = latest.lock().unwrap().take();
            // A still window sends no new frames; repeat the last one twice a second so a viewer
            // who joins late still gets a picture.
            let repeat = fresh.is_none() && last_encoded.elapsed() >= Duration::from_millis(500);
            if let Some(frame) = fresh {
                last = Some(frame);
            }
            if let Some(frame) = &last {
                if last_encoded.elapsed() >= period || repeat {
                    match encoder.encode(&frame.rgba, frame.width, frame.height) {
                        Ok(frames) => frames.into_iter().for_each(&sink),
                        Err(e) => tracing::warn!("{e:#}"),
                    }
                    last_encoded = Instant::now();
                }
            }
            std::thread::sleep(
                period
                    .saturating_sub(started.elapsed())
                    .max(Duration::from_millis(5)),
            );
        }
        let _ = stream.stop_capture();
    })
}
