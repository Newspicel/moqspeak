//! The status bar at the bottom of the window.

use zgui::prelude::*;
use zgui_ui::prelude::*;

use crate::audio::VoiceMode;
use crate::engine::{ConnStatus, MediaStatus};
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
        MediaStatus::Off => "Voice off".to_owned(),
        MediaStatus::Connecting => "Voice connecting…".to_owned(),
        MediaStatus::Connected { .. } => "Voice connected".to_owned(),
        MediaStatus::Failed(_) => "Voice unavailable".to_owned(),
    };
    let mode = move || match state.voice_mode.get() {
        VoiceMode::Activation => "Voice activation",
        VoiceMode::PushToTalk => "Push-to-talk",
        VoiceMode::Continuous => "Continuous",
    };
    let sending = move || {
        state
            .me
            .get()
            .is_some_and(|me| state.talking.with(|t| t.contains(&me)))
    };

    // One dot per link: green when up, amber while connecting, red when it failed.
    let conn_state = move || match state.status.get() {
        ConnStatus::Connected => "ok",
        ConnStatus::Connecting(_) => "busy",
        ConnStatus::Failed(_) => "bad",
        ConnStatus::Disconnected => "",
    };
    let media_state = move || match state.media.get() {
        MediaStatus::Connected { .. } => "ok",
        MediaStatus::Connecting => "busy",
        MediaStatus::Failed(_) => "bad",
        MediaStatus::Off => "",
    };

    view! {
        row(class = "statusbar") {
            row(class = "status-chip") {
                box(class = "status-dot", class:ok = move || conn_state() == "ok", class:busy = move || conn_state() == "busy", class:bad = move || conn_state() == "bad") {}
                text {{conn}}
            }
            Separator(orientation = SeparatorOrientation::Vertical, class = "status-sep")
            row(class = "status-chip") {
                box(class = "status-dot", class:ok = move || media_state() == "ok", class:busy = move || media_state() == "busy", class:bad = move || media_state() == "bad") {}
                text {{media}}
            }
            spacer()
            text(class = "status-mode") {{mode}}
            if move || state.voice_mode.get() == VoiceMode::PushToTalk {
                Kbd {"F1"}
            }
            box(class = "status-led", class:on = sending, a11y:label = "Transmitting") {}
        }
    }
}
