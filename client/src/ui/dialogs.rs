//! Connect, create channel, options, poke and about dialogs.

use zgui::prelude::*;

use crate::ui::IntoAny;
use zgui::reactive::{LocalStorage, RenderEffect, StoredValue};
use zgui_ui::prelude::*;

use crate::model::ClientMsg;
use crate::ui::options::OptionsDialogProps;
use crate::ui::state::{AppState, Modal};

/// Mounts whichever dialog the state asks for.
#[component]
pub fn Dialogs() -> impl IntoView {
    let state = AppState::expect();
    let kind = Memo::new(move |_| std::mem::discriminant(&state.modal.get()));
    view! {
        {move || {
            kind.track();
            match state.modal.get_untracked() {
                Modal::None => ().into_any(),
                Modal::Connect => view! { ConnectDialog() }.into_any(),
                Modal::CreateChannel { parent } => view! { CreateChannelDialog(parent = parent) }.into_any(),
                Modal::Options => view! { OptionsDialog() }.into_any(),
                Modal::Poke { to, name } => view! { PokeDialog(to = to, name = name) }.into_any(),
                Modal::Poked { from, text } => view! { PokedDialog(from = from, text = text) }.into_any(),
                Modal::About => view! { AboutDialog() }.into_any(),
                Modal::Share { monitors } => view! { ShareDialog(monitors = monitors) }.into_any(),
            }
        }}
    }
}

/// A dialog that is open while mounted and clears the modal when it closes.
fn open_binding(state: AppState) -> RwSignal<bool, LocalStorage> {
    let open = RwSignal::new_local(true);
    let effect = RenderEffect::new(move |_| {
        if !open.get() {
            state.modal.set(Modal::None);
        }
    });
    on_cleanup_local(move || drop(effect));
    open
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
fn ConnectDialog() -> impl IntoView {
    let state = AppState::expect();
    let open = open_binding(state);
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
        open.set(false);
    };
    let on_enter = move |ev: &mut EventCx<'_, zgui::view::events::KeyDown>| {
        if matches!(&ev.key, Key::Named(NamedKey::Enter)) {
            connect();
        }
    };

    view! {
        Dialog(open = open) {
            DialogContent(class = "ts-dialog") {
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
    }
}

#[component]
fn CreateChannelDialog(parent: Option<u64>) -> impl IntoView {
    let state = AppState::expect();
    let open = open_binding(state);
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
        open.set(false);
    };

    view! {
        Dialog(open = open) {
            DialogContent(class = "ts-dialog") {
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
    }
}

#[component]
fn PokeDialog(to: u64, name: String) -> impl IntoView {
    let state = AppState::expect();
    let open = open_binding(state);
    let text = RwSignal::new_local(String::new());
    let title = StoredValue::new(format!("Poke {name}"));
    let poke = move || {
        state.msg(ClientMsg::Poke {
            to,
            text: text.get_untracked(),
        });
        open.set(false);
    };
    view! {
        Dialog(open = open) {
            DialogContent(class = "ts-dialog") {
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
    }
}

#[component]
fn PokedDialog(from: String, text: String) -> impl IntoView {
    let state = AppState::expect();
    let open = open_binding(state);
    let title = StoredValue::new(format!("{from} poked you"));
    let body = StoredValue::new(if text.is_empty() {
        "(no message)".to_owned()
    } else {
        text
    });
    view! {
        Dialog(open = open) {
            DialogContent(class = "ts-dialog") {
                DialogHeader {
                    DialogTitle {{title.get_value()}}
                    DialogDescription {{body.get_value()}}
                }
                DialogFooter { DialogClose {"OK"} }
            }
        }
    }
}

#[component]
fn ShareDialog(monitors: Vec<crate::screen::MonitorInfo>) -> impl IntoView {
    let state = AppState::expect();
    let open = open_binding(state);
    let monitors = StoredValue::new(monitors);
    view! {
        Dialog(open = open) {
            DialogContent(class = "ts-dialog") {
                DialogHeader {
                    DialogTitle {"Share your screen"}
                    DialogDescription {"Everyone in your channel can watch. Pick a display."}
                }
                column(class = "monitor-list") {
                    for m in move || monitors.get_value(), key = |m: &crate::screen::MonitorInfo| m.id {
                        control(
                            class = "monitor",
                            tabindex = Focus::Sequential,
                            on:click = move |_| {
                                state.send(crate::engine::Command::StartShare { monitor: m.id });
                                open.set(false);
                            }
                        ) {
                            text(class = "monitor-name") {{format!("{}{}", m.name, if m.primary { " (main)" } else { "" })}}
                            text(class = "monitor-size") {{format!("{} × {}", m.width, m.height)}}
                        }
                    }
                }
                DialogFooter { DialogClose(variant = ButtonVariant::Outline) {"Cancel"} }
            }
        }
    }
}

#[component]
fn AboutDialog() -> impl IntoView {
    let state = AppState::expect();
    let open = open_binding(state);
    view! {
        Dialog(open = open) {
            DialogContent(class = "ts-dialog") {
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
    }
}
