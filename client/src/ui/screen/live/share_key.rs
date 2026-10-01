//! The pill key that starts and stops a screen share.

use std::rc::Rc;

use zgui::prelude::*;
use zgui_ui_primitives::Placement;

use crate::ui::parts::KeyProps;
use crate::ui::parts::icons;
use crate::ui::screen::live::toggle::toggle_share;
use crate::ui::state::AppState;

/// Shares the screen, or stops the share.
#[component]
pub fn ShareKey() -> impl IntoView {
    let state = AppState::expect();
    let share = Rc::new(move || toggle_share(state));
    view! {
        Key(
            svg = Signal::derive_local(move || if state.sharing.get() { icons::SCREEN_SHARE_OFF } else { icons::SCREEN_SHARE }),
            label = Signal::derive_local(move || if state.sharing.get() { "Stop sharing" } else { "Share screen" }.to_owned()),
            on = Signal::derive_local(move || state.sharing.get()),
            disabled = Signal::derive_local(move || !state.connected()),
            tone = "err",
            placement = Placement::TOP,
            on_press = share
        )
    }
}
