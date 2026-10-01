//! The tag on a client that shares its screen.

use zgui::prelude::*;
use zgui::reactive::LocalStorage;

use crate::ui::parts::TagProps;
use crate::ui::state::ClientRow;

/// "live", while the client shares its screen.
#[component]
pub fn LiveTag(row: Signal<ClientRow, LocalStorage>) -> impl IntoView {
    view! {
        if move || row.with(|r| r.sharing) {
            Tag(text = "live", tone = "err")
        }
    }
}
