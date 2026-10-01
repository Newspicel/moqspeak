//! The connect field: a server address and a nickname in one frame, and the key that connects.

use zgui::prelude::*;
use zgui_ui::prelude::*;

use crate::engine::ConnStatus;
use crate::ui::parts::icons::{self, IconProps, IconSize};
use crate::ui::state::AppState;

/// The two lines, the key, and what the last attempt came to.
#[component]
pub fn ConnectField() -> impl IntoView {
    let state = AppState::expect();
    let settings = state.settings.get_untracked();
    let address = RwSignal::new_local(settings.address);
    let nickname = RwSignal::new_local(settings.nickname);
    // A saved server chosen below writes its address into the field.
    let follow = zgui::reactive::RenderEffect::new(move |_| {
        let saved = state.settings.with(|s| s.address.clone());
        if address.get_untracked() != saved {
            address.set(saved);
        }
    });
    on_cleanup_local(move || drop(follow));

    let connect = move || {
        let a = address.get_untracked().trim().to_owned();
        let n = nickname.get_untracked().trim().to_owned();
        if !a.is_empty() && !n.is_empty() {
            state.connect(a, n);
        }
    };
    let on_enter = move |ev: &mut EventCx<'_, events::KeyDown>| {
        if matches!(&ev.key, Key::Named(NamedKey::Enter)) {
            connect();
        }
    };
    let connecting = move || matches!(state.status.get(), ConnStatus::Connecting(_));
    let status = move || match state.status.get() {
        ConnStatus::Connecting(server) => Some((format!("Connecting to {server}…"), "")),
        ConnStatus::Failed(e) => Some((format!("Connection failed: {e}"), "err")),
        _ => None,
    };

    view! {
        column(class = "ms-connect") {
            row(class = "ms-connect__line") {
                text(class = "ms-connect__label") {"Server"}
                Input(value = address, label = "Server address", placeholder = "host/server", class = "ms-connect__input", on:key_down = on_enter)
            }
            row(class = "ms-connect__line") {
                text(class = "ms-connect__label") {"Nickname"}
                Input(value = nickname, label = "Nickname", class = "ms-connect__input", on:key_down = on_enter)
            }
        }
        row(class = "ms-home__actions") {
            row(
                class = "ms-home__status",
                attr:data-tone = move || status().map(|(_, tone)| tone.to_owned()).filter(|t| !t.is_empty())
            ) {
                if move || connecting() {
                    box(class = "ms-busy") { Icon(svg = icons::LOADER_CIRCLE, size = IconSize::Xs) }
                }
                text(class = "ms-home__status-text") {{move || status().map(|(text, _)| text).unwrap_or_default()}}
            }
            Button(variant = ButtonVariant::Outline, size = ButtonSize::Sm, class = "ms-accent-key", on:click = move |_| connect()) {"Connect"}
        }
    }
}
