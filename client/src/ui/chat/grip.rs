//! The grip on the leading edge of the chat column that drags its width.

use zgui::prelude::*;
use zgui::reactive::LocalStorage;

use crate::ui::state::{AppState, CHAT_MAX, CHAT_MIN};

/// How far one arrow key moves the edge, in CSS pixels.
const STEP: f32 = 16.0;

/// The width a drag that started at `from` with the column `was` wide gives at `at`. The column
/// grows as the edge moves towards the leading side.
pub fn dragged(from: f32, was: f32, at: f32) -> f32 {
    (was + from - at).clamp(CHAT_MIN, CHAT_MAX)
}

/// A thin strip along the edge. A drag moves the edge; the arrow keys move it a step.
#[component]
pub fn ChatGrip(
    /// Whether a drag is under way, which holds the column's width transition.
    resizing: RwSignal<bool, LocalStorage>,
    /// The width the column stands at, which a drag and a step start from.
    shown: Signal<f32, LocalStorage>,
) -> impl IntoView {
    let state = AppState::expect();
    let from: RwSignal<Option<(f32, f32)>, LocalStorage> = RwSignal::new_local(None);
    let set = move |width: f32| state.chat_width.set(width.clamp(CHAT_MIN, CHAT_MAX));
    let save = move || {
        let width = state.chat_width.get_untracked();
        state.update_settings(|s| s.chat_width = width);
    };
    let end = move || {
        if from.get_untracked().is_some() {
            from.set(None);
            resizing.set(false);
            save();
        }
    };

    view! {
        control(
            class = "ms-chat__grip",
            tabindex = Focus::Sequential,
            a11y:role = Role::Splitter,
            a11y:label = "Chat width",
            on:pointer_down = move |ev| {
                if ev.button != Some(PointerButton::Primary) {
                    return;
                }
                ev.capture_pointer();
                ev.stop_propagation();
                from.set(Some((f32::from(ev.position.x), shown.get_untracked())));
                resizing.set(true);
            },
            on:pointer_move = move |ev| {
                if let Some((start, was)) = from.get_untracked() {
                    set(dragged(start, was, f32::from(ev.position.x)));
                }
            },
            on:pointer_up = move |ev| {
                ev.release_pointer();
                end();
            },
            on:pointer_cancel = move |ev| {
                ev.release_pointer();
                end();
            },
            on:key_down = move |ev| {
                let width = shown.get_untracked();
                match &ev.key {
                    Key::Named(NamedKey::ArrowLeft) => set(width + STEP),
                    Key::Named(NamedKey::ArrowRight) => set(width - STEP),
                    _ => return,
                }
                ev.prevent_default();
                save();
            }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::dragged;
    use crate::ui::state::{CHAT_MAX, CHAT_MIN};

    #[test]
    fn dragging_the_edge_towards_the_tree_widens_the_column() {
        assert_eq!(dragged(800.0, 360.0, 760.0), 400.0);
        assert_eq!(dragged(800.0, 360.0, 840.0), 320.0);
    }

    #[test]
    fn a_drag_keeps_the_column_in_its_range() {
        assert_eq!(dragged(800.0, 360.0, 0.0), CHAT_MAX);
        assert_eq!(dragged(800.0, 360.0, 2000.0), CHAT_MIN);
    }
}
