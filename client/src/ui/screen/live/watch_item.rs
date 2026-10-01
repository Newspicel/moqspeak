//! The client menu item that opens a shared screen in its own window.

use zgui::prelude::*;
use zgui::reactive::LocalStorage;
use zgui_ui::prelude::*;

use crate::model::ClientId;
use crate::ui::parts::act;
use crate::ui::parts::icons::{self, IconProps, IconSize};
use crate::ui::screen::live::viewer::pop_out;
use crate::ui::state::{AppState, ClientRow};

/// "Watch screen", while the client shares its screen.
#[component]
pub fn WatchItem(id: ClientId, row: Signal<ClientRow, LocalStorage>) -> impl IntoView {
    let state = AppState::expect();
    view! {
        if move || row.with(|r| r.sharing) {
            MenuItem(on_select = act(move || pop_out(state, id, row.with_untracked(|r| r.name.clone())))) {
                Icon(svg = icons::MONITOR_PLAY, size = IconSize::Sm)
                text {"Watch screen"}
            }
        }
    }
}
