//! Who draws the frame around the window.
//!
//! macOS keeps its frame for a window that asks for no title bar, so the window buttons, the
//! corners and the resize edges belong to the system. The buttons stand over the leading corner of
//! the content. Windows, X11 and Wayland hand the whole frame over, and the window draws its own
//! keys and resize edges.

use zgui::prelude::*;

use crate::ui::parts::Erase;

/// Who draws the frame around the window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameSource {
    /// The desktop draws the frame. Its window buttons stand over the leading corner.
    System,
    /// The window draws its own keys and resize edges.
    Own,
}

impl FrameSource {
    /// The source on this desktop.
    pub const CURRENT: Self = if cfg!(target_os = "macos") {
        Self::System
    } else {
        Self::Own
    };

    /// The token CSS selects on.
    pub const fn token(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Own => "own",
        }
    }
}

/// Draws `view` where the window draws its own frame.
pub fn own_frame(view: impl IntoView + 'static) -> AnyView {
    match FrameSource::CURRENT {
        FrameSource::Own => view.any(),
        FrameSource::System => ().any(),
    }
}
