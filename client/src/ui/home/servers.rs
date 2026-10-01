//! The saved servers: a press connects, the cross forgets one.

use std::rc::Rc;

use zgui::prelude::*;

use crate::ui::parts::icons;
use crate::ui::parts::{KeyProps, StatusDotProps, press};
use crate::ui::state::{AppState, Bookmark};

/// One saved server.
#[component]
fn ServerEntry(bookmark: Bookmark) -> impl IntoView {
    let state = AppState::expect();
    let address = bookmark.address.clone();
    let forget = Rc::new(move || state.remove_bookmark(&address));
    let label = bookmark.label.clone();
    let shown = bookmark.address.clone();
    let by_key = bookmark.clone();
    view! {
        row(
            class = "ms-servers__row",
            tabindex = Focus::Sequential,
            a11y:role = Role::Button,
            a11y:label = format!("Connect to {label}"),
            on:pointer_down = press::press(),
            on:click = move |_| state.connect_bookmark(&bookmark),
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
                svg = Signal::stored_local(icons::X),
                label = Signal::stored_local("Forget server".to_owned()),
                on_press = forget
            )
        }
    }
}

/// Every saved server.
#[component]
pub fn ServerList() -> impl IntoView {
    let state = AppState::expect();
    let bookmarks = move || state.settings.with(|s| s.bookmarks.clone());
    view! {
        column(class = "ms-servers") {
            row(class = "ms-servers__title") {
                text {"Servers"}
                text {{move || bookmarks().len().to_string()}}
            }
            if move || bookmarks().is_empty() {
                text(class = "ms-servers__empty") {"Servers you save from the server menu show up here."}
            }
            for bookmark in move || bookmarks(), key = |b: &Bookmark| b.clone() {
                ServerEntry(bookmark = bookmark)
            }
        }
    }
}
