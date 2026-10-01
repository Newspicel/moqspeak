//! How the window is laid out: the server column, the chat beside it, and the overlays.
//!
//! The frame also hears the keys and pointer moves every region shares: the push-to-talk key,
//! Escape, and the drag of a client onto a channel.

use zgui::prelude::*;

use crate::ui::chat::ChatPanelProps;
use crate::ui::dialogs::DialogsProps;
use crate::ui::frame::controls::WindowControlsProps;
use crate::ui::frame::edges::ResizeEdgesProps;
use crate::ui::frame::ghost::DragGhostProps;
use crate::ui::head::HeadProps;
use crate::ui::home::HomeProps;
use crate::ui::log::LogDockProps;
use crate::ui::pill::PillProps;
use crate::ui::settings::SettingsViewProps;
use crate::ui::state::{AppState, Drag};
use crate::ui::tree::TreeProps;

/// Whether a key is the push-to-talk key: F1 anywhere, or backtick outside text fields.
fn is_ptt(key: &Key) -> bool {
    match key {
        Key::Named(NamedKey::F1) => true,
        Key::Character(c) => c.as_str() == "`" && !focus_is_text_entry(),
        _ => false,
    }
}

/// Everything inside the window.
#[component]
pub fn Frame() -> impl IntoView {
    let state = AppState::expect();
    let online = move || state.online();
    let settings = move || state.settings_open.get();
    let chat_open = move || state.chat.with(|c| c.open) && online() && !settings();

    view! {
        stack(
            class = "ms-frame",
            attr:data-chat = move || chat_open().then(|| "open".to_owned()),
            attr:data-platform = cfg!(target_os = "macos").then(|| "macos".to_owned()),
            on:key_down = move |ev| {
                if is_ptt(&ev.key) {
                    state.set_ptt(true);
                    ev.prevent_default();
                }
            },
            on:key_up = move |ev| {
                if is_ptt(&ev.key) {
                    state.set_ptt(false);
                }
                if matches!(&ev.key, Key::Named(NamedKey::Escape)) {
                    if state.dragging() {
                        state.cancel_drag();
                    } else if state.settings_open.get_untracked() {
                        state.settings_open.set(false);
                    }
                }
            },
            on:pointer_move = move |ev| {
                let Some(drag) = state.drag.get_untracked() else { return };
                let at = (f32::from(ev.position.x), f32::from(ev.position.y));
                if !drag.active {
                    let (dx, dy) = (at.0 - drag.origin.0, at.1 - drag.origin.1);
                    if dx * dx + dy * dy > 36.0 {
                        state.drag.set(Some(Drag { active: true, ..drag }));
                    }
                }
                state.pointer.set(at);
            },
            on:pointer_up = move |_| state.finish_drag(),
            // A release can be lost when the row it targeted was rebuilt mid-press; the next
            // press ends whatever drag is still hanging.
            on:pointer_down = move |_| {
                if state.drag.with_untracked(|d| d.as_ref().is_some_and(|d| d.active)) {
                    state.cancel_drag();
                }
            }
        ) {
            row(class = "ms-body") {
                if move || settings() {
                    SettingsView()
                } else {
                    column(class = "ms-main") {
                        Head()
                        if move || online() {
                            column(class = "ms-stage") {
                                Tree()
                                Pill()
                            }
                            if move || state.log_open.get() {
                                LogDock()
                            }
                        } else {
                            Home()
                        }
                    }
                    ChatPanel(open = Signal::derive_local(chat_open))
                }
            }
            WindowControls()
            ResizeEdges()
            DragGhost()
            Dialogs()
        }
    }
}
