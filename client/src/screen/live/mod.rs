//! Screen sharing as the `screen-share` feature builds it: capture with xcap, AV1 with rav1e,
//! decode with rav1d.
//!
//! A shared screen is the `screen` track of the sharer's broadcast. Every keyframe starts a new
//! MoQ group, so a viewer who subscribes late begins at the latest keyframe.

mod color;
mod decode;
mod encode;
mod frame;
#[cfg(target_os = "macos")]
mod handoff;
#[cfg(target_os = "macos")]
pub mod mac;
mod monitor;
mod obu;
mod share;
#[cfg(test)]
mod tests;

pub use decode::Decoder;
pub use encode::{Sharer, list_monitors};
pub use frame::VideoFrame;
#[cfg(target_os = "macos")]
pub use handoff::Handoff;
pub use monitor::MonitorInfo;
pub use share::{Share, ShareEvent, Source};
