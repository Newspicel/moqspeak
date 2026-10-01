//! The right-hand panel: details of whatever the tree has selected.

use zgui::prelude::*;

use crate::ui::IntoAny;
use zgui::reactive::UnsyncCallback;
use zgui_ui::prelude::*;

use crate::engine::{ConnStatus, MediaStatus};
use crate::model::{ChannelId, ClientId};
use crate::ui::avatar::AvatarProps;
use crate::ui::icons::{self, IcoProps};
use crate::ui::state::{AppState, Modal, Selection};

fn ago(ms: u64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let secs = now.saturating_sub(ms) / 1000;
    match secs {
        0..=59 => format!("{secs} seconds"),
        60..=3599 => format!("{} minutes", secs / 60),
        3600..=86399 => format!("{} hours {} minutes", secs / 3600, (secs % 3600) / 60),
        _ => format!("{} days", secs / 86400),
    }
}

/// A label and a value on one line.
#[component]
fn Field(#[prop(into)] name: String, value: Signal<String>) -> impl IntoView {
    view! {
        row(class = "info-field") {
            text(class = "info-key") {{name.clone()}}
            text(class = "info-value") {{move || value.get()}}
        }
    }
}

#[component]
pub fn InfoPanel() -> impl IntoView {
    let state = AppState::expect();
    view! {
        column(class = "pane info-pane") {
            ScrollArea(class = "info-scroll", label = "Information") {
                column(class = "info") {
                    {move || match state.selected.get() {
                        _ if !state.connected() => view! { Welcome() }.into_any(),
                        Selection::Server => view! { ServerInfoView() }.into_any(),
                        Selection::Channel(id) => view! { ChannelInfo(id = id) }.into_any(),
                        Selection::Client(id) => view! { ClientInfo(id = id) }.into_any(),
                    }}
                }
            }
        }
    }
}

#[component]
fn Welcome() -> impl IntoView {
    view! {
        column(class = "banner") {
            text(class = "banner-title") {"moqspeak"}
            text(class = "banner-sub") {"Media over QUIC Speak"}
        }
        label(class = "info-text") {
            "A native voice client built with zgui. The control plane runs on a Cloudflare Worker; \
             voice travels as Opus over Cloudflare's draft-16 MoQ relay."
        }
        label(class = "info-text") {"Use Connections → Connect, or the bookmark star in the toolbar, to join a server."}
    }
}

#[component]
fn ServerInfoView() -> impl IntoView {
    let state = AppState::expect();
    let s = state.server;
    view! {
        column(class = "banner") {
            text(class = "banner-title") {{move || s.with(|s| s.name.clone())}}
            text(class = "banner-sub") {"moqspeak server"}
        }
        column(class = "info-fields") {
            Field(name = "Address:", value = Signal::derive(move || state.settings.with(|s| s.address.clone())))
            Field(name = "Clients online:", value = Signal::derive(move || state.clients.with(Vec::len).to_string()))
            Field(name = "Channels:", value = Signal::derive(move || state.channels.with(Vec::len).to_string()))
            Field(name = "Server age:", value = Signal::derive(move || ago(s.with(|s| s.created_at))))
            Field(name = "Control plane:", value = Signal::derive(|| "Cloudflare Worker + Durable Object".to_owned()))
            Field(name = "Voice transport:", value = Signal::derive(move || match state.media.get() {
                MediaStatus::Off => "off".to_owned(),
                MediaStatus::Connecting => "connecting…".to_owned(),
                MediaStatus::Connected { .. } => "MoQ draft-16 (Cloudflare relay)".to_owned(),
                MediaStatus::Failed(e) => format!("failed: {e}"),
            }))
            Field(name = "Codec:", value = Signal::derive(|| "Opus Voice, 48 kHz mono, 40 kbit/s".to_owned()))
        }
        if move || s.with(|s| !s.welcome.is_empty()) {
            column(class = "info-block") {
                text(class = "info-heading") {"Welcome message"}
                label(class = "info-text") {{move || s.with(|s| s.welcome.clone())}}
            }
        }
    }
}

#[component]
fn ChannelInfo(id: ChannelId) -> impl IntoView {
    let state = AppState::expect();
    let ch = Memo::new(move |_| state.channel(id));
    let count = move || {
        state
            .clients
            .with(|c| c.iter().filter(|c| c.channel == id).count())
    };
    let mine = move || state.my_client().is_some_and(|c| c.channel == id);
    view! {
        column(class = "banner banner-channel") {
            row(class = "banner-row") {
                Ico(svg = icons::CHANNEL)
                text(class = "banner-title") {{move || ch.get().map(|c| c.name).unwrap_or_default()}}
            }
            text(class = "banner-sub") {{move || ch.get().map(|c| if c.topic.is_empty() { "No topic".to_owned() } else { c.topic }).unwrap_or_default()}}
        }
        column(class = "info-fields") {
            Field(name = "Clients:", value = Signal::derive(move || {
                let max = ch.get().map(|c| c.max_clients).unwrap_or(0);
                if max > 0 { format!("{} / {max}", count()) } else { format!("{} / unlimited", count()) }
            }))
            Field(name = "Type:", value = Signal::derive(move || {
                if ch.get().is_some_and(|c| c.is_default) { "Permanent, default".to_owned() } else { "Permanent".to_owned() }
            }))
            Field(name = "Codec:", value = Signal::derive(|| "Opus Voice".to_owned()))
            Field(name = "Media:", value = Signal::derive(|| "one MoQ broadcast per speaker, track \"audio\"".to_owned()))
        }
        if move || ch.get().is_some_and(|c| !c.description.is_empty()) {
            column(class = "info-block") {
                text(class = "info-heading") {"Description"}
                label(class = "info-text") {{move || ch.get().map(|c| c.description).unwrap_or_default()}}
            }
        }
        row(class = "info-actions") {
            if move || !mine() {
                control(class = "btn btn-primary", tabindex = Focus::Sequential, on:click = move |_| state.join(id)) {
                    "Switch to channel"
                }
            }
            control(
                class = "btn",
                tabindex = Focus::Sequential,
                on:click = move |_| state.modal.set(Modal::CreateChannel { parent: Some(id) })
            ) {"Create sub-channel"}
        }
    }
}

#[component]
fn ClientInfo(id: ClientId) -> impl IntoView {
    let state = AppState::expect();
    let c = Memo::new(move |_| state.client(id));
    let me = move || state.me.get() == Some(id);
    let talking = move || state.talking.with(|t| t.contains(&id));
    let volume = RwSignal::new_local(
        state
            .volumes
            .with_untracked(|v| v.get(&id).copied().unwrap_or(1.0)) as f64
            * 100.0,
    );
    let locally_muted = move || state.local_mutes.with(|m| m.contains(&id));

    view! {
        column(class = "banner banner-client", class:talking = talking) {
            row(class = "banner-row") {
                Avatar(name = c.get_untracked().map(|c| c.name).unwrap_or_default(), talking = Signal::derive(talking), size = "lg")
                text(class = "banner-title") {{move || c.get().map(|c| c.name).unwrap_or_default()}}
            }
            text(class = "banner-sub") {{move || {
                if talking() { "Talking".to_owned() }
                else if let Some(c) = c.get() {
                    if c.away { format!("Away {}", c.away_message) }
                    else if c.muted { "Microphone muted".to_owned() }
                    else if c.deaf { "Speakers muted".to_owned() }
                    else { "Online".to_owned() }
                } else { String::new() }
            }}}
        }
        column(class = "info-fields") {
            Field(name = "Channel:", value = Signal::derive(move || {
                c.get().and_then(|c| state.channel(c.channel)).map(|ch| ch.name).unwrap_or_default()
            }))
            Field(name = "Online for:", value = Signal::derive(move || c.get().map(|c| ago(c.connected_at)).unwrap_or_default()))
            Field(name = "Version:", value = Signal::derive(move || c.get().map(|c| format!("moqspeak {} on {}", c.version, c.platform)).unwrap_or_default()))
            Field(name = "Client ID:", value = Signal::derive(move || id.to_string()))
            Field(name = "Broadcast:", value = Signal::derive(move || c.get().map(|c| c.broadcast).unwrap_or_default()))
        }
        if move || !me() {
            column(class = "info-block") {
                text(class = "info-heading") {"Local volume"}
                row(class = "volume-row") {
                    Slider(
                        value = volume,
                        min = 0.0,
                        max = 200.0,
                        step = 5.0,
                        label = "Volume",
                        class = "volume-slider",
                        on_change = UnsyncCallback::new(move |v: f64| state.set_volume(id, (v / 100.0) as f32))
                    )
                    text(class = "volume-value") {{move || format!("{:.0}%", volume.get())}}
                }
            }
            row(class = "info-actions") {
                control(class = "btn btn-primary", tabindex = Focus::Sequential, on:click = move |_| state.open_private(id)) {
                    "Send message"
                }
                control(class = "btn", tabindex = Focus::Sequential, on:click = move |_| {
                    let name = c.get_untracked().map(|c| c.name).unwrap_or_default();
                    state.modal.set(Modal::Poke { to: id, name });
                }) {"Poke"}
                control(class = "btn", tabindex = Focus::Sequential, on:click = move |_| state.set_local_mute(id, !locally_muted())) {
                    {move || if locally_muted() { "Unmute locally" } else { "Mute locally" }}
                }
            }
        }
        if move || state.status.with(|s| *s != ConnStatus::Connected) {
            label {"Disconnected"}
        }
    }
}
