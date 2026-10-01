//! Poke a client: a short message that pops up on their screen.

use zgui::prelude::*;
use zgui::reactive::StoredValue;
use zgui_ui::prelude::*;

use crate::model::ClientMsg;
use crate::ui::state::{AppState, Modal};

/// The message field.
#[component]
pub fn PokeBody() -> impl IntoView {
    let state = AppState::expect();
    let (to, name) = match state.modal.get_untracked() {
        Modal::Poke { to, name } => (to, name),
        _ => (0, String::new()),
    };
    let text = RwSignal::new_local(String::new());
    let title = StoredValue::new(format!("Poke {name}"));
    let poke = move || {
        state.msg(ClientMsg::Poke {
            to,
            text: text.get_untracked(),
        });
        state.modal.set(Modal::None);
    };
    view! {
        DialogHeader {
            DialogTitle {{title.get_value()}}
            DialogDescription {"The message pops up on their screen."}
        }
        Input(value = text, label = "Poke message", placeholder = "Message",
            on:key_down = move |ev| if matches!(&ev.key, Key::Named(NamedKey::Enter)) { poke() })
        DialogFooter {
            DialogClose(variant = ButtonVariant::Outline) {"Cancel"}
            Button(on:click = move |_| poke()) {"Poke"}
        }
    }
}
