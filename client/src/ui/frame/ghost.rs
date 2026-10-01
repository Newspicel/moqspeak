//! The label that follows the pointer while a client is dragged onto a channel.

use zgui::prelude::*;

use crate::ui::state::AppState;

/// Who is moved, and where to, beside the pointer.
#[component]
pub fn DragGhost() -> impl IntoView {
    let state = AppState::expect();
    let name = move || {
        state
            .drag
            .with(|d| d.as_ref().map(|d| d.name.clone()).unwrap_or_default())
    };
    let target = move || {
        state
            .drop_target
            .get()
            .and_then(|c| state.channel(c))
            .map(|c| c.name)
    };
    view! {
        if move || state.dragging() {
            row(
                class = "ms-ghost",
                style:left = move || Some(format!("{}px", state.pointer.get().0 + 14.0)),
                style:top = move || Some(format!("{}px", state.pointer.get().1 + 10.0))
            ) {
                text {{name}}
                if move || target().is_some() {
                    text {"→"}
                    text(class = "ms-ghost__to") {{move || target().unwrap_or_default()}}
                }
            }
        }
    }
}
