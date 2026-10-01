//! The status bar at the bottom of the window.

use zgui::prelude::*;

use crate::audio::VoiceMode;
use crate::engine::{ConnStatus, MediaStatus};
use crate::ui::icons::{self, IcoProps};
use crate::ui::state::AppState;

#[component]
pub fn StatusBar() -> impl IntoView {
    let state = AppState::expect();
    let conn = move || match state.status.get() {
        ConnStatus::Disconnected => "Disconnected".to_owned(),
        ConnStatus::Connecting(s) => format!("Connecting to {s}…"),
        ConnStatus::Failed(e) => format!("Connection failed: {e}"),
        ConnStatus::Connected => match state.my_client() {
            Some(c) => format!("Connected as {}", c.name),
            None => "Connected".to_owned(),
        },
    };
    let media = move || match state.media.get() {
        MediaStatus::Off => "Voice: off".to_owned(),
        MediaStatus::Connecting => "Voice: connecting to MoQ relay…".to_owned(),
        MediaStatus::Connected {
            relay,
            subscriptions,
        } => {
            format!("Voice: MoQ draft-16 · {relay} · listening to {subscriptions}")
        }
        MediaStatus::Failed(e) => format!("Voice failed: {e}"),
    };
    let mode = move || match state.voice_mode.get() {
        VoiceMode::Activation => "Voice activation",
        VoiceMode::PushToTalk => "Push-to-talk (F1)",
        VoiceMode::Continuous => "Continuous",
    };
    let sending = move || {
        state
            .me
            .get()
            .is_some_and(|me| state.talking.with(|t| t.contains(&me)))
    };

    let conn_dot = move || match state.status.get() {
        ConnStatus::Connected => "status-dot ok",
        ConnStatus::Connecting(_) => "status-dot busy",
        ConnStatus::Failed(_) => "status-dot bad",
        ConnStatus::Disconnected => "status-dot",
    };
    let media_dot = move || match state.media.get() {
        MediaStatus::Connected { .. } => "status-dot ok",
        MediaStatus::Connecting => "status-dot busy",
        MediaStatus::Failed(_) => "status-dot bad",
        MediaStatus::Off => "status-dot",
    };

    view! {
        row(class = "statusbar") {
            row(class = "status-chip", class:ok = move || state.connected()) {
                box(class = "status-dot", class:ok = move || conn_dot() == "status-dot ok", class:busy = move || conn_dot() == "status-dot busy", class:bad = move || conn_dot() == "status-dot bad") {}
                text {{conn}}
            }
            box(class = "status-sep") {}
            row(class = "status-chip", class:ok = move || matches!(state.media.get(), MediaStatus::Connected { .. })) {
                box(class = "status-dot", class:ok = move || media_dot() == "status-dot ok", class:busy = move || media_dot() == "status-dot busy", class:bad = move || media_dot() == "status-dot bad") {}
                Ico(svg = icons::RELAY)
                text {{media}}
            }
            spacer()
            text(class = "status-mode") {{mode}}
            box(class = "status-led", class:on = sending, a11y:label = "Transmitting") {}
        }
    }
}
