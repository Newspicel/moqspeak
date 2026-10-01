//! The row at the top of the server column. The whole row moves the window.

use std::rc::Rc;

use zgui::prelude::*;

use crate::ui::head::keys::HeadKeysProps;
use crate::ui::head::server_menu::ServerMenuProps;
use crate::ui::head::where_::WhereProps;
use crate::ui::parts::icons;
use crate::ui::parts::{KeyProps, press};
use crate::ui::state::AppState;

/// The server, where you are in it, and the keys for what the window shows. Without a server it
/// holds only the way to the settings.
#[component]
pub fn Head() -> impl IntoView {
    let state = AppState::expect();
    let settings = Rc::new(move || state.settings_open.set(true));
    view! {
        row(class = "ms-head", on:pointer_down = press::move_window()) {
            if move || state.online() {
                ServerMenu()
                Where()
                box(class = "ms-head__gap")
                HeadKeys()
            } else {
                box(class = "ms-head__gap")
                Key(
                    svg = Signal::stored_local(icons::SETTINGS),
                    label = Signal::stored_local("Settings".to_owned()),
                    on_press = settings.clone()
                )
            }
        }
    }
}
