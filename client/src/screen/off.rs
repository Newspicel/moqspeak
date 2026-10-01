//! The share a session runs in a build without the `screen-share` feature. It never starts and
//! never reports.

/// What a running share reports. This build has nothing to report.
pub enum ShareEvent {}

/// A share that never runs.
#[derive(Default)]
pub struct Share {}

impl Share {
    /// Waits forever.
    pub async fn next(&mut self) -> ShareEvent {
        std::future::pending().await
    }
}
