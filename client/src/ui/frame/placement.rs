//! How the window stands on the screen: in its own bounds, maximised or full screen.

use zgui::prelude::*;
use zgui::reactive::LocalStorage;

/// How the window stands on the screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Placement {
    /// The window has its own bounds and resize edges.
    Normal,
    /// The window fills the work area.
    Maximized,
    /// The window fills the screen.
    FullScreen,
}

impl Placement {
    /// The placement from the window's two flags. Full screen outranks maximised.
    pub const fn of(maximized: bool, full_screen: bool) -> Self {
        match (maximized, full_screen) {
            (_, true) => Self::FullScreen,
            (true, false) => Self::Maximized,
            (false, false) => Self::Normal,
        }
    }

    /// The placement of the current window. A tree outside a window stays normal.
    pub fn watch() -> Signal<Self, LocalStorage> {
        let Some(window) = try_use_window() else {
            return Signal::stored_local(Self::Normal);
        };
        let (maximized, full_screen) = (window.maximized(), window.fullscreen());
        Signal::derive_local(move || Self::of(maximized.get(), full_screen.get().is_some()))
    }

    /// The token CSS selects on. A normal window carries none.
    pub const fn token(self) -> Option<&'static str> {
        match self {
            Self::Normal => None,
            Self::Maximized => Some("max"),
            Self::FullScreen => Some("full"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_screen_outranks_maximised() {
        assert_eq!(Placement::of(false, false), Placement::Normal);
        assert_eq!(Placement::of(true, false), Placement::Maximized);
        assert_eq!(Placement::of(false, true), Placement::FullScreen);
        assert_eq!(Placement::of(true, true), Placement::FullScreen);
    }

    #[test]
    fn only_a_placed_window_carries_a_token() {
        assert_eq!(Placement::Normal.token(), None);
        assert_eq!(Placement::Maximized.token(), Some("max"));
        assert_eq!(Placement::FullScreen.token(), Some("full"));
    }
}
