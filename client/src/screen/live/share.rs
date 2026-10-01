//! The share a session runs: the capture, the frames it yields, and its end.

use std::time::Duration;

use anyhow::Result;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use tokio::time::{Interval, MissedTickBehavior};

#[cfg(target_os = "macos")]
use super::{Handoff, mac};
use super::{Sharer, VideoFrame};

/// What to share.
#[derive(Clone, Debug)]
pub enum Source {
    /// One monitor, by id.
    Monitor(u32),
    /// A moving test pattern, for headless testing.
    TestPattern,
    /// What the user chose in the macOS system picker.
    #[cfg(target_os = "macos")]
    Picked(Handoff<mac::Picked>),
}

/// What a running share reports.
pub enum ShareEvent {
    /// A frame to publish.
    Frame(VideoFrame),
    /// The system ended the capture, as macOS does from its menu bar.
    Ended,
}

/// The capture a session shares, if any.
pub struct Share {
    sharer: Option<Sharer>,
    sink: UnboundedSender<VideoFrame>,
    frames: UnboundedReceiver<VideoFrame>,
    check: Interval,
}

impl Default for Share {
    /// A share that runs nothing yet. It is made inside a tokio runtime.
    fn default() -> Self {
        let (sink, frames) = unbounded_channel();
        let mut check = tokio::time::interval(Duration::from_millis(250));
        check.set_missed_tick_behavior(MissedTickBehavior::Delay);
        Self {
            sharer: None,
            sink,
            frames,
            check,
        }
    }
}

impl Share {
    /// Shares `source` and replaces any running share.
    pub fn start(&mut self, source: Source) -> Result<()> {
        self.sharer = None;
        let tx = self.sink.clone();
        let sink = move |frame| {
            let _ = tx.send(frame);
        };
        self.sharer = Some(match source {
            Source::Monitor(id) => Sharer::start(id, sink),
            Source::TestPattern => Sharer::test_pattern(sink),
            #[cfg(target_os = "macos")]
            Source::Picked(picked) => {
                let picked = picked
                    .take()
                    .ok_or_else(|| anyhow::anyhow!("that choice was already used"))?;
                mac::start(picked, sink)
            }
        }?);
        Ok(())
    }

    /// Stops the running share. Returns whether one ran.
    pub fn stop(&mut self) -> bool {
        self.sharer.take().is_some()
    }

    /// Waits for the next frame, or for the system to end the capture.
    pub async fn next(&mut self) -> ShareEvent {
        loop {
            tokio::select! {
                Some(frame) = self.frames.recv() => return ShareEvent::Frame(frame),
                _ = self.check.tick() => {
                    if self.sharer.as_ref().is_some_and(Sharer::ended) {
                        self.sharer = None;
                        return ShareEvent::Ended;
                    }
                }
            }
        }
    }
}
