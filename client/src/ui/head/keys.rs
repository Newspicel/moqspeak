//! The keys at the trailing end of the head: a new channel, the log and the chat.

use std::rc::Rc;

use zgui::prelude::*;

use crate::model::Role;
use crate::ui::parts::KeyProps;
use crate::ui::parts::icons;
use crate::ui::state::{AppState, Modal, Selection};

/// The keys a server shows.
#[component]
pub fn HeadKeys() -> impl IntoView {
    let state = AppState::expect();
    let create = Rc::new(move || {
        let parent = match state.selected.get_untracked() {
            Selection::Channel(id) => Some(id),
            _ => None,
        };
        state.modal.set(Modal::CreateChannel { parent });
    });
    let log = Rc::new(move || state.log_open.update(|open| *open = !*open));
    let chat = Rc::new(move || state.chat.update(|c| c.toggle()));
    let unread = Signal::derive_local(move || state.chat.with(|c| !c.open && !c.unread.is_empty()));

    view! {
        if move || state.my_role() >= Role::Mod {
            Key(
                svg = Signal::stored_local(icons::FOLDER_PLUS),
                label = Signal::stored_local("New channel".to_owned()),
                on_press = create.clone()
            )
        }
        Key(
            svg = Signal::stored_local(icons::SCROLL_TEXT),
            label = Signal::derive_local(move || if state.log_open.get() { "Hide log" } else { "Show log" }.to_owned()),
            on = Signal::derive_local(move || state.log_open.get()),
            on_press = log
        )
        Key(
            svg = Signal::stored_local(icons::MESSAGE_SQUARE),
            label = Signal::derive_local(move || if state.chat.with(|c| c.open) { "Hide chat" } else { "Show chat" }.to_owned()),
            on = Signal::derive_local(move || state.chat.with(|c| c.open)),
            dot = unread,
            on_press = chat
        )
    }
}
