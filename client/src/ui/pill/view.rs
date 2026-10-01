//! The pill: your own voice controls, floating over the foot of the tree.

use std::rc::Rc;

use zgui::prelude::*;
use zgui_ui_primitives::Placement;

use crate::audio::VoiceMode;
use crate::ui::parts::KeyProps;
use crate::ui::parts::icons;
use crate::ui::pill::self_menu::SelfMenuProps;
use crate::ui::pill::talk_key::TalkKeyProps;
use crate::ui::state::AppState;

/// You, the microphone, the sound, away, push-to-talk, the screen and the settings.
#[component]
pub fn Pill() -> impl IntoView {
    let state = AppState::expect();
    let mic = Rc::new(move || state.set_mic_muted(!state.mic_muted.get_untracked()));
    let sound = Rc::new(move || state.set_deafened(!state.deafened.get_untracked()));
    let away = Rc::new(move || state.set_away(!state.away.get_untracked()));
    let share = Rc::new(move || state.toggle_share());
    let settings = Rc::new(move || state.settings_open.set(true));
    let label = |text: &'static str| Signal::stored_local(text.to_owned());

    view! {
        row(class = "ms-pill-seat") {
            row(class = "ms-pill", a11y:role = Role::Toolbar, a11y:label = "Your voice") {
                SelfMenu()
                box(class = "ms-pill__sep")
                Key(
                    svg = Signal::derive_local(move || if state.mic_muted.get() { icons::MIC_OFF } else { icons::MIC }),
                    label = Signal::derive_local(move || if state.mic_muted.get() { "Unmute microphone" } else { "Mute microphone" }.to_owned()),
                    on = Signal::derive_local(move || state.mic_muted.get()),
                    tone = "err",
                    placement = Placement::TOP,
                    on_press = mic
                )
                Key(
                    svg = Signal::derive_local(move || if state.deafened.get() { icons::HEADPHONE_OFF } else { icons::HEADPHONES }),
                    label = Signal::derive_local(move || if state.deafened.get() { "Turn sound on" } else { "Turn sound off" }.to_owned()),
                    on = Signal::derive_local(move || state.deafened.get()),
                    tone = "err",
                    placement = Placement::TOP,
                    on_press = sound
                )
                Key(
                    svg = Signal::stored_local(icons::MOON),
                    label = Signal::derive_local(move || if state.away.get() { "Back from away" } else { "Set away" }.to_owned()),
                    on = Signal::derive_local(move || state.away.get()),
                    tone = "warn",
                    placement = Placement::TOP,
                    on_press = away
                )
                if move || state.voice_mode.get() == VoiceMode::PushToTalk {
                    TalkKey()
                }
                box(class = "ms-pill__sep")
                Key(
                    svg = Signal::derive_local(move || if state.sharing.get() { icons::SCREEN_SHARE_OFF } else { icons::SCREEN_SHARE }),
                    label = Signal::derive_local(move || if state.sharing.get() { "Stop sharing" } else { "Share screen" }.to_owned()),
                    on = Signal::derive_local(move || state.sharing.get()),
                    disabled = Signal::derive_local(move || !state.connected()),
                    tone = "err",
                    placement = Placement::TOP,
                    on_press = share
                )
                Key(
                    svg = Signal::stored_local(icons::SETTINGS),
                    label = label("Settings"),
                    placement = Placement::TOP,
                    on_press = settings
                )
            }
        }
    }
}
