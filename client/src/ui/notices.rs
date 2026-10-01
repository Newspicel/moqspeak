//! Moves the announcements the state collects onto the toaster.

use zgui::prelude::*;
use zgui::reactive::RenderEffect;
use zgui_ui::prelude::*;

use crate::ui::state::{AppState, Level};

/// Shows each note as a toast. A note that names a client offers a reply.
#[component]
pub fn Notices() -> impl IntoView {
    let state = AppState::expect();
    let queue = use_toaster();
    let draining = RenderEffect::new(move |_| {
        let notes = state.notes.get();
        if notes.is_empty() {
            return;
        }
        state.notes.set(Vec::new());
        let Some(queue) = queue else { return };
        for note in notes {
            let mut toast = Toast::new(note.title).kind(match note.level {
                Level::Info => ToastKind::Normal,
                Level::Error => ToastKind::Error,
            });
            if let Some(detail) = note.detail {
                toast = toast.description(detail);
            }
            if let Some(id) = note.reply {
                toast = toast.action("Reply", move || state.open_direct(id));
            }
            queue.push(toast);
        }
    });
    on_cleanup_local(move || drop(draining));
    view! { box(style:display = "none") }
}
