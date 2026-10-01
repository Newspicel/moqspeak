//! The window layout: menu, toolbar, tree and info side by side, chat, status bar.

use zgui::prelude::*;

use crate::ui::chat::ChatPanelProps;
use crate::ui::dialogs::DialogsProps;
use crate::ui::info::InfoPanelProps;
use crate::ui::menu::MainMenuProps;
use crate::ui::state::{AppState, Drag, Theme};
use crate::ui::status::StatusBarProps;
use crate::ui::toolbar::ToolbarProps;
use crate::ui::tree::ServerTreeProps;

/// Whether a key press is the push-to-talk key: F1 anywhere, or backtick outside text fields.
fn is_ptt(key: &Key) -> bool {
    match key {
        Key::Named(NamedKey::F1) => true,
        Key::Character(c) => c.as_str() == "`" && !focus_is_text_entry(),
        _ => false,
    }
}

#[component]
pub fn Shell() -> impl IntoView {
    let state = AppState::expect();

    view! {
        column(
            class = "shell",
            class:dark = move || state.theme.get() == Theme::Dark,
            class:light = move || state.theme.get() == Theme::Light,
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
                    state.drag.set(None);
                    state.drop_target.set(None);
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
            class:dragging = move || state.drag.with(|d| d.as_ref().is_some_and(|d| d.active))
        ) {
            MainMenu()
            Toolbar()
            row(class = "main") {
                ServerTree()
                InfoPanel()
            }
            ChatPanel()
            StatusBar()
            Dialogs()
            if move || state.drag.with(|d| d.as_ref().is_some_and(|d| d.active)) {
                row(
                    class = "drag-ghost",
                    style:left = move || Some(format!("{}px", state.pointer.get().0 + 14.0)),
                    style:top = move || Some(format!("{}px", state.pointer.get().1 + 10.0))
                ) {
                    text {{move || {
                        let name = state.drag.with(|d| d.as_ref().map(|d| d.name.clone()).unwrap_or_default());
                        match state.drop_target.get().and_then(|c| state.channel(c)) {
                            Some(ch) => format!("Move {name} to {}", ch.name),
                            None => format!("Move {name}…"),
                        }
                    }}}
                }
            }
        }
    }
}
