//! The servers joined before: a press connects again, the cross forgets one.

use std::rc::Rc;

use zgui::prelude::*;

use crate::ui::parts::icons;
use crate::ui::parts::{KeyProps, StatusDotProps, press};
use crate::ui::state::{AppState, Bookmark};

/// One server from the history.
#[component]
fn RecentEntry(entry: Bookmark) -> impl IntoView {
    let state = AppState::expect();
    let address = entry.address.clone();
    let forget = Rc::new(move || state.forget_server(&address));
    let saved_address = entry.address.clone();
    let saved_label = entry.label.clone();
    let save = Rc::new(move || state.save_server(&saved_label, &saved_address));
    let label = entry.label.clone();
    let shown = entry.address.clone();
    let by_key = entry.clone();
    view! {
        row(
            class = "ms-servers__row",
            tabindex = Focus::Sequential,
            a11y:role = Role::Button,
            a11y:label = format!("Connect to {label}"),
            on:pointer_down = press::press(),
            on:click = move |_| state.connect_bookmark(&entry),
            on:key_down = move |ev| {
                if matches!(&ev.key, Key::Named(NamedKey::Enter)) {
                    state.connect_bookmark(&by_key);
                }
            }
        ) {
            StatusDot(tone = Signal::stored_local("idle"))
            text(class = "ms-servers__label") {{label.clone()}}
            text(class = "ms-servers__address") {{shown}}
            Key(
                svg = Signal::stored_local(icons::STAR),
                label = Signal::stored_local("Save server".to_owned()),
                on_press = save
            )
            Key(
                svg = Signal::stored_local(icons::X),
                label = Signal::stored_local("Forget server".to_owned()),
                on_press = forget
            )
        }
    }
}

/// Every server joined before, newest first.
#[component]
pub fn RecentList() -> impl IntoView {
    let state = AppState::expect();
    let history = move || state.settings.with(|s| s.history.clone());
    view! {
        if move || !history().is_empty() {
            column(class = "ms-servers") {
                row(class = "ms-servers__title") {
                    text {"Recent"}
                    text {{move || history().len().to_string()}}
                }
                for entry in move || history(), key = |b: &Bookmark| b.clone() {
                    RecentEntry(entry = entry)
                }
            }
        }
    }
}
