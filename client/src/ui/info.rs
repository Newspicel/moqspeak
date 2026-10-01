//! The right-hand panel: details of whatever the tree has selected, as zgui-ui cards.

use zgui::prelude::*;

use crate::ui::IntoAny;
use zgui::reactive::UnsyncCallback;
use zgui_ui::prelude::*;

use crate::engine::MediaStatus;
use crate::model::{ChannelId, ClientId};
use crate::ui::avatar::UserAvatarProps;
use crate::ui::icons::{self, IcoProps};
use crate::ui::screen::ScreenViewProps;
use crate::ui::state::{AppState, Selection};

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
        Card(class = "info-card hero") {
            CardHeader {
                CardTitle {"moqspeak"}
                CardDescription {"Media over QUIC Speak"}
            }
            CardContent {
                label(class = "info-text") {"Connect to a server to talk with your friends."}
            }
        }
    }
}

#[component]
fn ServerInfoView() -> impl IntoView {
    let state = AppState::expect();
    let s = state.server;
    view! {
        Card(class = "info-card hero") {
            CardHeader {
                CardTitle {
                    row(class = "card-title-row") {
                        Ico(svg = icons::SERVER, class = "srv-ico")
                        text {{move || s.with(|s| s.name.clone())}}
                    }
                }
                CardDescription {{move || state.settings.with(|s| s.address.clone())}}
                CardAction {
                    Badge(variant = BadgeVariant::Secondary) {{move || format!("{} online", state.clients.with(Vec::len))}}
                }
            }
            CardContent {
                column(class = "info-fields") {
                    Field(name = "Channels", value = Signal::derive(move || state.channels.with(Vec::len).to_string()))
                    Field(name = "Server age", value = Signal::derive(move || ago(s.with(|s| s.created_at))))
                    Field(name = "Voice", value = Signal::derive(move || match state.media.get() {
                        MediaStatus::Off => "Off".to_owned(),
                        MediaStatus::Connecting => "Connecting…".to_owned(),
                        MediaStatus::Connected { .. } => "Connected".to_owned(),
                        MediaStatus::Failed(_) => "Unavailable".to_owned(),
                    }))
                }
            }
        }
        if move || s.with(|s| !s.welcome.is_empty()) {
            Card(class = "info-card") {
                CardHeader { CardTitle {"Welcome message"} }
                CardContent { label(class = "info-text") {{move || s.with(|s| s.welcome.clone())}} }
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
    view! {
        Card(class = "info-card hero") {
            CardHeader {
                CardTitle {
                    row(class = "card-title-row") {
                        Ico(svg = icons::CHANNEL, class = "ch-ico")
                        text {{move || ch.get().map(|c| c.name).unwrap_or_default()}}
                    }
                }
                CardDescription {{move || ch.get().map(|c| if c.topic.is_empty() { "No topic".to_owned() } else { c.topic }).unwrap_or_default()}}
                CardAction {
                    if move || ch.get().is_some_and(|c| c.is_default) {
                        Badge(variant = BadgeVariant::Secondary) {"Default"}
                    }
                }
            }
            CardContent {
                column(class = "info-fields") {
                    Field(name = "Clients", value = Signal::derive(move || {
                        let max = ch.get().map(|c| c.max_clients).unwrap_or(0);
                        if max > 0 { format!("{} / {max}", count()) } else { format!("{} / unlimited", count()) }
                    }))
                }
            }
        }
        if move || ch.get().is_some_and(|c| !c.description.is_empty()) {
            Card(class = "info-card") {
                CardHeader { CardTitle {"Description"} }
                CardContent { label(class = "info-text") {{move || ch.get().map(|c| c.description).unwrap_or_default()}} }
            }
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
    let watching = RwSignal::new_local(true);
    let status = move || {
        if talking() {
            ("Talking", BadgeVariant::Default)
        } else if let Some(c) = c.get() {
            if c.away {
                ("Away", BadgeVariant::Outline)
            } else if c.muted {
                ("Mic muted", BadgeVariant::Destructive)
            } else if c.deaf {
                ("Sound muted", BadgeVariant::Destructive)
            } else {
                ("Online", BadgeVariant::Success)
            }
        } else {
            ("Offline", BadgeVariant::Outline)
        }
    };

    view! {
        Card(class = "info-card hero", class:talking = talking) {
            CardHeader {
                CardTitle {
                    row(class = "card-title-row") {
                        UserAvatar(name = c.get_untracked().map(|c| c.name).unwrap_or_default(), talking = Signal::derive(talking), size = AvatarSize::Lg)
                        column(class = "card-title-text") {
                            text {{move || c.get().map(|c| c.name).unwrap_or_default()}}
                            text(class = "card-sub") {{move || c.get().and_then(|c| state.channel(c.channel)).map(|ch| format!("in {}", ch.name)).unwrap_or_default()}}
                        }
                    }
                }
                CardAction {
                    {move || {
                        let (label, variant) = status();
                        view! { Badge(variant = variant) {{label}} }
                    }}
                }
            }
        }
        if move || c.get().is_some_and(|c| c.sharing) && !me() {
            Card(class = "info-card") {
                CardHeader {
                    CardTitle {"Screen"}
                    CardAction {
                        row(class = "card-actions") {
                            Badge(variant = BadgeVariant::Destructive) {"LIVE"}
                            Button(variant = ButtonVariant::Outline, size = ButtonSize::Sm, on:click = move |_| watching.update(|w| *w = !*w)) {
                                {move || if watching.get() { "Hide" } else { "Watch" }}
                            }
                            Button(size = ButtonSize::Sm, on:click = move |_| {
                                let name = c.get_untracked().map(|c| c.name).unwrap_or_default();
                                crate::ui::screen::pop_out(state, id, name);
                            }) {"Open in window"}
                        }
                    }
                }
                if move || watching.get() {
                    CardContent {
                        ScreenView(state = state, client = id, class = "screen-frame")
                    }
                }
            }
        }
        if move || me() && state.sharing.get() {
            Card(class = "info-card") {
                CardHeader {
                    CardTitle {"Screen"}
                    CardDescription {"You are sharing your screen with your channel."}
                }
            }
        }
        if move || !me() {
            Card(class = "info-card") {
                CardHeader {
                    CardTitle {"Volume"}
                    CardDescription {"Only you hear this change."}
                }
                CardContent {
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
            }
        }
        Card(class = "info-card") {
            CardHeader { CardTitle {"Details"} }
            CardContent {
                column(class = "info-fields") {
                    Field(name = "Online for", value = Signal::derive(move || c.get().map(|c| ago(c.connected_at)).unwrap_or_default()))
                    Field(name = "Client", value = Signal::derive(move || c.get().map(|c| format!("moqspeak {} on {}", c.version, c.platform)).unwrap_or_default()))
                }
            }
        }
    }
}
