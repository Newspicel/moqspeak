//! The field at the foot of the chat that sends a message on Enter.

use zgui::prelude::*;
use zgui_ui::prelude::*;

use crate::ui::parts::Erase;
use crate::ui::state::{AppState, Conversation};

/// One framed line. Enter sends what it holds to the conversation the panel shows.
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
        }
    }
}
