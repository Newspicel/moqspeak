//! You, at the leading end of the pill: the mark that lights while you transmit, your name, and
//! a menu for the rest of your own state.

use zgui::prelude::*;
use zgui::reactive::LocalStorage;
use zgui_ui::prelude::*;

use crate::audio::VoiceMode;
use crate::ui::parts::icons::{self, IconProps, IconSize};
use crate::ui::parts::{StatusDotProps, act, press};
use crate::ui::state::AppState;

/// Your name and your voice mark, with the menu of your own state.
#[component]
pub fn SelfMenu() -> impl IntoView {
    let state = AppState::expect();
    let me = state.me;
    let tone: Signal<&'static str, LocalStorage> = Signal::derive_local(move || {
        let talking = me
            .get()
            .is_some_and(|id| state.talking.with(|t| t.contains(&id)));
        if talking {
            "talk"
        } else if state.away.get() {
            "warn"
        } else {
            "idle"
        }
    });
    let name = move || {
        state
            .my_client()
            .map(|c| c.name)
            .unwrap_or_else(|| state.settings.with(|s| s.nickname.clone()))
    };

    view! {
        DropdownMenu {
            DropdownMenuTrigger(variant = ButtonVariant::Ghost, size = ButtonSize::Sm, class = "ms-self", on:pointer_down = press::hold()) {
                StatusDot(tone = tone)
                text(class = "ms-self__name") {{name}}
                if move || state.voice_mode.get() == VoiceMode::PushToTalk {
                    text(class = "ms-self__kbd") {"F1"}
                }
            }
            DropdownMenuContent(placement = Signal::stored_local(zgui_ui_primitives::Placement::TOP)) {
                MenuLabel {{move || match state.voice_mode.get() {
                    VoiceMode::Activation => "Voice activation",
                    VoiceMode::PushToTalk => "Push-to-talk · hold F1",
                    VoiceMode::Continuous => "Continuous transmission",
                }}}
                MenuItem(on_select = act(move || state.set_away(!state.away.get_untracked()))) {
                    Icon(svg = icons::MOON, size = IconSize::Sm)
                    text {{move || if state.away.get() { "Back from away" } else { "Set away" }}}
                }
                MenuItem(on_select = act(move || state.set_loopback(!state.loopback.get_untracked()))) {
                    Icon(svg = icons::EAR, size = IconSize::Sm)
                    text {{move || if state.loopback.get() { "Stop hearing myself" } else { "Hear myself" }}}
                }
                MenuSeparator()
                MenuItem(destructive = true, on_select = act(move || state.disconnect())) {
                    Icon(svg = icons::UNPLUG, size = IconSize::Sm)
                    text {"Disconnect"}
                }
            }
        }
    }
}
