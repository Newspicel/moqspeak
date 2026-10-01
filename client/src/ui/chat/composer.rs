//! The field at the foot of the chat that sends a message on Enter.

use zgui::prelude::*;
use zgui_ui::prelude::*;

use std::rc::Rc;

use zgui_ui_primitives::Placement;

use crate::ui::parts::icons;
use crate::ui::parts::{Erase, KeyProps};
use crate::ui::state::{AppState, Conversation};

/// One framed line and a send key. Enter or the key sends what it holds to the conversation the
/// panel shows.
#[component]
pub fn Composer() -> impl IntoView {
    let state = AppState::expect();
    let draft = RwSignal::new_local(String::new());
    let send = move || {
        let text = draft.get_untracked();
        if text.trim().is_empty() {
            return;
        }
        state.send_chat(text);
        draft.set(String::new());
    };
    let placeholder = Signal::derive_local(move || match state.chat.with(|c| c.current) {
        Conversation::Channel => state
            .my_channel()
            .map(|c| format!("Message #{}", c.name))
            .unwrap_or_else(|| "Message the channel".into()),
        Conversation::Server => "Message everyone".into(),
        Conversation::Private(id) => state
            .client(id)
            .map(|c| format!("Message {}", c.name))
            .unwrap_or_else(|| "Message".into()),
    });
    let offline = Signal::derive_local(move || !state.connected());
    let ready =
        Signal::derive_local(move || !draft.with(|d| d.trim().is_empty()) && state.connected());
    let press: Rc<dyn Fn()> = Rc::new(send);

    // The placeholder names the conversation, so the field is built again when it changes.
    view! {
        row(class = "ms-composer") {
            {move || view! { Input(
                value = draft,
                placeholder = placeholder.get(),
                disabled = offline,
                label = "Message",
                class = "ms-composer__input",
                on:key_down = move |ev| {
                    if matches!(&ev.key, Key::Named(NamedKey::Enter)) {
                        send();
                    }
                }
            ) }.any()}
            Key(
                svg = Signal::stored_local(icons::SEND_HORIZONTAL),
                label = Signal::stored_local("Send".to_owned()),
                on = ready,
                disabled = Signal::derive_local(move || !ready.get()),
                placement = Placement::TOP,
                class = "ms-composer__send",
                on_press = press
            )
        }
    }
}
