//! Connect to another server while one is open.

use zgui::prelude::*;
use zgui_ui::prelude::*;

use crate::ui::dialogs::host::FormRowProps;
use crate::ui::state::{AppState, Modal};

/// The address and nickname fields.
#[component]
pub fn ConnectBody() -> impl IntoView {
    let state = AppState::expect();
    let settings = state.settings.get_untracked();
    let address = RwSignal::new_local(settings.address);
    let nickname = RwSignal::new_local(settings.nickname);
    let connect = move || {
        let a = address.get_untracked().trim().to_owned();
        let n = nickname.get_untracked().trim().to_owned();
        if a.is_empty() || n.is_empty() {
            return;
        }
        state.connect(a, n);
        state.modal.set(Modal::None);
    };
    let on_enter = move |ev: &mut EventCx<'_, events::KeyDown>| {
        if matches!(&ev.key, Key::Named(NamedKey::Enter)) {
            connect();
        }
    };
    view! {
        DialogHeader {
            DialogTitle {"Connect"}
            DialogDescription {"The address is the host and the server name, such as moq.newspicel.dev/public."}
        }
        column(class = "ms-form") {
            FormRow(label = "Server") {
                Input(value = address, label = "Server address", on:key_down = on_enter)
            }
            FormRow(label = "Nickname") {
                Input(value = nickname, label = "Nickname", on:key_down = on_enter)
            }
        }
        DialogFooter {
            DialogClose(variant = ButtonVariant::Outline) {"Cancel"}
            Button(on:click = move |_| connect()) {"Connect"}
        }
    }
}
