//! How wide the chat column stands in a window of a given width.

use zgui::prelude::*;
use zgui::reactive::LocalStorage;

use crate::ui::state::{AppState, CHAT_MIN};

/// The width the server column keeps beside the chat, in CSS pixels.
const SERVER_MIN: f32 = 320.0;

/// The width of the column: the chosen width, narrowed so the server column keeps `SERVER_MIN`.
/// The column never narrows below `CHAT_MIN`.
pub fn fitted(chosen: f32, window: f32) -> f32 {
    chosen.min(window - SERVER_MIN).max(CHAT_MIN)
}

/// The width the column stands at in the current window.
pub fn shown_width(state: AppState) -> Signal<f32, LocalStorage> {
    let window = try_use_window().map(|w| w.size());
    Signal::derive_local(move || {
        let chosen = state.chat_width.get();
        match window {
            Some(size) => fitted(chosen, size.get().width.0),
            None => chosen,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_wide_window_keeps_the_chosen_width() {
        assert_eq!(fitted(360.0, 1100.0), 360.0);
    }

    #[test]
    fn a_narrow_window_narrows_the_column_for_the_server() {
        assert_eq!(fitted(600.0, 800.0), 480.0);
        assert_eq!(fitted(600.0, 500.0), CHAT_MIN);
    }
}
