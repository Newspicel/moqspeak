//! The screen-sharing pieces in a build without the `screen-share` feature. Each draws nothing.

use zgui::prelude::*;
use zgui::reactive::LocalStorage;

use crate::model::ClientId;
use crate::ui::state::ClientRow;

/// The share key of the pill.
#[component]
pub fn ShareKey() -> impl IntoView {}

/// The display picker.
#[component]
pub fn ShareDialog() -> impl IntoView {}

/// The menu item that opens a client's shared screen.
#[component]
pub fn WatchItem(id: ClientId, row: Signal<ClientRow, LocalStorage>) -> impl IntoView {
    let _ = (id, row);
}

/// The tag on a client that shares its screen.
#[component]
pub fn LiveTag(row: Signal<ClientRow, LocalStorage>) -> impl IntoView {
    let _ = row;
}
