//! Connect, create channel, options, poke, share and about dialogs.
//!
//! Every dialog stays mounted and opens through its `open` binding, which follows
//! [`AppState::modal`]. Its body is rebuilt each time it opens, so its fields start fresh.

use zgui::prelude::*;

use zgui::reactive::{LocalStorage, RenderEffect, StoredValue, UnsyncCallback};
use zgui_ui::prelude::*;

use crate::model::ClientMsg;
use crate::ui::options::OptionsBodyProps;
use crate::ui::state::{AppState, Modal};

/// Hosts one dialog, open while `is` matches the current modal.
#[component]
fn ModalHost(
    is: fn(&Modal) -> bool,
    #[prop(into)] class: String,
    children: ChildrenFn,
) -> impl IntoView {
    let state = AppState::expect();
    let open: RwSignal<bool, LocalStorage> = RwSignal::new_local(false);
    let sync = RenderEffect::new(move |_| {
        let want = state.modal.with(|m| is(m));
        if open.get_untracked() != want {
            open.set(want);
        }
    });
    on_cleanup_local(move || drop(sync));
    let on_open_change = UnsyncCallback::new(move |next: bool| {
        if !next && state.modal.with_untracked(|m| is(m)) {
            state.modal.set(Modal::None);
        }
    });
    let class = StoredValue::new(class);
    view! {
        Dialog(open = open, on_open_change = on_open_change) {
            DialogContent(class = class.get_value()) {
                {children.view()}
            }
        }
    }
}

/// Every dialog the application has.
#[component]
pub fn Dialogs() -> impl IntoView {
    view! {
        ModalHost(is = |m| matches!(m, Modal::Connect), class = "ts-dialog") { ConnectBody() }
        ModalHost(is = |m| matches!(m, Modal::CreateChannel { .. }), class = "ts-dialog") { CreateChannelBody() }
        ModalHost(is = |m| matches!(m, Modal::Options), class = "ts-dialog ts-options") { OptionsBody() }
        ModalHost(is = |m| matches!(m, Modal::Poke { .. }), class = "ts-dialog") { PokeBody() }
        ModalHost(is = |m| matches!(m, Modal::Poked { .. }), class = "ts-dialog") { PokedBody() }
        ModalHost(is = |m| matches!(m, Modal::Share { .. }), class = "ts-dialog") { ShareBody() }
        ModalHost(is = |m| matches!(m, Modal::About), class = "ts-dialog") { AboutBody() }
    }
}

/// A label above a control.
#[component]
fn Row(#[prop(into)] label: String, children: Children) -> impl IntoView {
    view! {
        column(class = "form-row") {
            text(class = "form-label") {{label}}
            {children.into_view_once()}
        }
    }
}

#[component]
fn ConnectBody() -> impl IntoView {
    let state = AppState::expect();
    let settings = state.settings.get_untracked();
    let address = RwSignal::new_local(settings.address.clone());
    let nickname = RwSignal::new_local(settings.nickname.clone());

    let connect = move || {
        let a = address.get_untracked().trim().to_owned();
        let n = nickname.get_untracked().trim().to_owned();
        if a.is_empty() || n.is_empty() {
            return;
        }
        state.connect(a, n);
        state.modal.set(Modal::None);
    };
    let on_enter = move |ev: &mut EventCx<'_, zgui::view::events::KeyDown>| {
        if matches!(&ev.key, Key::Named(NamedKey::Enter)) {
            connect();
        }
    };

    view! {
                DialogHeader {
                    DialogTitle {"Connect"}
                    DialogDescription {"Server address is the Worker host and server name, e.g. moqspeak.newspicel.workers.dev/public"}
                }
                column(class = "form") {
                    Row(label = "Server Address") {
                        Input(value = address, label = "Server address", on:key_down = on_enter)
                    }
                    Row(label = "Nickname") {
                        Input(value = nickname, label = "Nickname", on:key_down = on_enter)
                    }
                }
                DialogFooter {
                    DialogClose(variant = ButtonVariant::Outline) {"Cancel"}
                    Button(on:click = move |_| connect()) {"Connect"}
                }
    }
}

#[component]
fn CreateChannelBody() -> impl IntoView {
    let state = AppState::expect();
    let parent = match state.modal.get_untracked() {
        Modal::CreateChannel { parent } => parent,
        _ => None,
    };
    let name = RwSignal::new_local(String::new());
    let topic = RwSignal::new_local(String::new());
    let description = RwSignal::new_local(String::new());
    let max = RwSignal::new_local(String::new());
    let subtitle = match parent.and_then(|p| state.channel(p)) {
        Some(p) => format!("New sub-channel of \"{}\"", p.name),
        None => "New top-level channel".to_owned(),
    };
    let subtitle = StoredValue::new(subtitle);

    let create = move || {
        let n = name.get_untracked().trim().to_owned();
        if n.is_empty() {
            return;
        }
        state.msg(ClientMsg::CreateChannel {
            name: n,
            topic: topic.get_untracked(),
            description: description.get_untracked(),
            parent,
            max_clients: max.get_untracked().trim().parse().unwrap_or(0),
        });
        state.modal.set(Modal::None);
    };

    view! {
                DialogHeader {
                    DialogTitle {"Create Channel"}
                    DialogDescription {{subtitle.get_value()}}
                }
                column(class = "form") {
                    Row(label = "Name") { Input(value = name, label = "Channel name") }
                    Row(label = "Topic") { Input(value = topic, label = "Topic") }
                    Row(label = "Description") { Textarea(value = description, label = "Description") }
                    Row(label = "Maximum clients (empty = unlimited)") { Input(value = max, label = "Maximum clients") }
                }
                DialogFooter {
                    DialogClose(variant = ButtonVariant::Outline) {"Cancel"}
                    Button(on:click = move |_| create()) {"Create"}
                }
    }
}

#[component]
fn PokeBody() -> impl IntoView {
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

#[component]
fn PokedBody() -> impl IntoView {
    let state = AppState::expect();
    let (from, text) = match state.modal.get_untracked() {
        Modal::Poked { from, text } => (from, text),
        _ => (String::new(), String::new()),
    };
    let title = StoredValue::new(format!("{from} poked you"));
    let body = StoredValue::new(if text.is_empty() {
        "(no message)".to_owned()
    } else {
        text
    });
    view! {
                DialogHeader {
                    DialogTitle {{title.get_value()}}
                    DialogDescription {{body.get_value()}}
                }
                DialogFooter { DialogClose {"OK"} }
    }
}

#[component]
fn ShareBody() -> impl IntoView {
    let state = AppState::expect();
    let monitors = match state.modal.get_untracked() {
        Modal::Share { monitors } => monitors,
        _ => Vec::new(),
    };
    let monitors = StoredValue::new(monitors);
    view! {
                DialogHeader {
                    DialogTitle {"Share your screen"}
                    DialogDescription {"Everyone in your channel can watch. Pick a display."}
                }
                column(class = "monitor-list") {
                    for m in move || monitors.get_value(), key = |m: &crate::screen::MonitorInfo| m.id {
                        Item(variant = ItemVariant::Outline) {
                            ItemContent {
                                ItemTitle {{format!("{}{}", m.name, if m.primary { " (main)" } else { "" })}}
                                ItemDescription {{format!("{} × {}", m.width, m.height)}}
                            }
                            ItemActions {
                                Button(size = ButtonSize::Sm, on:click = move |_| {
                                    state.send(crate::engine::Command::StartShare { monitor: m.id });
                                    state.modal.set(Modal::None);
                                }) {"Share"}
                            }
                        }
                    }
                }
                DialogFooter { DialogClose(variant = ButtonVariant::Outline) {"Cancel"} }
    }
}

#[component]
fn AboutBody() -> impl IntoView {
    view! {
                DialogHeader {
                    DialogTitle {"About moqspeak"}
                    DialogDescription {{format!("Version {}", env!("CARGO_PKG_VERSION"))}}
                }
                column(class = "about") {
                    label {"Media over QUIC Speak: a TeamSpeak-style voice client written in Rust with the zgui framework."}
                    label {"Control: Cloudflare Worker with a Durable Object per server."}
                    label {"Voice: Opus over Media over QUIC (moq-net, IETF draft-16) through Cloudflare's MoQ relay."}
                }
                DialogFooter { DialogClose {"Close"} }
    }
}
