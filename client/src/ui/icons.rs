//! The icon set: Lucide (ISC licence, see `assets/icons/LICENSE`).

#![allow(
    dead_code,
    reason = "the set holds icons for features that come and go"
)]

use zgui::prelude::*;

/// Each icon is a Lucide SVG drawn with `currentColor`, so CSS decides its colour.
macro_rules! lucide {
    ($file:literal) => {
        include_str!(concat!("../../assets/icons/", $file, ".svg"))
    };
}

pub const SERVER: &str = lucide!("globe");
pub const CHANNEL: &str = lucide!("message-square");
pub const HOME: &str = lucide!("house");
pub const MIC: &str = lucide!("mic");
pub const MIC_MUTED: &str = lucide!("mic-off");
pub const SPEAKER: &str = lucide!("headphones");
pub const SPEAKER_MUTED: &str = lucide!("headphone-off");
pub const AWAY: &str = lucide!("moon");
pub const CONNECT: &str = lucide!("plug");
pub const DISCONNECT: &str = lucide!("unplug");
pub const BOOKMARK: &str = lucide!("star");
pub const SETTINGS: &str = lucide!("settings");
pub const CHAT: &str = lucide!("message-square");
pub const POKE: &str = lucide!("hand");
pub const ADD_CHANNEL: &str = lucide!("folder-plus");
pub const INFO: &str = lucide!("info");
pub const ERROR: &str = lucide!("circle-alert");
pub const RELAY: &str = lucide!("zap");
pub const SCREEN: &str = lucide!("screen-share");
pub const SCREEN_OFF: &str = lucide!("screen-share-off");
pub const MONITOR: &str = lucide!("monitor");
pub const KICK: &str = lucide!("user-x");
pub const VOLUME: &str = lucide!("volume-2");
pub const PLUS: &str = lucide!("plus");
pub const CLOSE: &str = lucide!("x");
pub const HASH: &str = lucide!("hash");
pub const LEAVE: &str = lucide!("log-out");
pub const AUDIO: &str = lucide!("audio-lines");
pub const CHEVRON: &str = lucide!("chevron-right");

/// A 16-pixel icon drawn from one of the documents above.
#[component]
pub fn Ico(
    /// The SVG document.
    svg: &'static str,
    #[prop(into, optional)] class: Option<String>,
) -> impl IntoView {
    let class = class
        .map(|c| format!("ico {c}"))
        .unwrap_or_else(|| "ico".into());
    view! {
        vector(class = class, prop:svg = svg, a11y:hidden = true)
    }
}

/// An icon whose document follows a signal.
#[component]
pub fn DynIco(svg: Signal<&'static str>) -> impl IntoView {
    view! {
        vector(class = "ico", prop:svg = move || PropValue::from(svg.get()), a11y:hidden = true)
    }
}
