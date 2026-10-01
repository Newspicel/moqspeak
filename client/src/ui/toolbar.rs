//! The icon toolbar under the menu.

use zgui::prelude::*;

use crate::audio::VoiceMode;
use crate::engine::ConnStatus;
use crate::ui::icons::{self, DynIcoProps, IcoProps};
use crate::ui::state::{AppState, Modal};

/// One toolbar button.
#[component]
fn Tool(
    svg: &'static str,
    #[prop(into)] label: String,
    #[prop(default = "")] tone: &'static str,
    #[prop(into, optional)] active: Option<Signal<bool>>,
    #[prop(into, optional)] disabled: Option<Signal<bool>>,
    on_press: std::rc::Rc<dyn Fn()>,
) -> impl IntoView {
    let active = active.unwrap_or_else(|| Signal::stored(false));
    let disabled = disabled.unwrap_or_else(|| Signal::stored(false));
    view! {
        control(
            class = format!("tool {tone}"),
            class:active = move || active.get(),
            class:disabled = move || disabled.get(),
            tabindex = Focus::Sequential,
            a11y:label = label.clone(),
            attr:title = Some(label.clone()),
            on:click = move |_| if !disabled.get_untracked() { on_press() }
        ) {
            Ico(svg = svg)
        }
    }
}

#[component]
pub fn Toolbar() -> impl IntoView {
    let state = AppState::expect();
    let connected = Signal::derive(move || state.connected());
    let offline = Signal::derive(move || {
        state
            .status
            .with(|s| matches!(s, ConnStatus::Disconnected | ConnStatus::Failed(_)))
    });
    let not_connected = Signal::derive(move || !connected.get());
    let mic = Signal::derive(move || {
        if state.mic_muted.get() {
            icons::MIC_MUTED
        } else {
            icons::MIC
        }
    });
    let speaker = Signal::derive(move || {
        if state.deafened.get() {
            icons::SPEAKER_MUTED
        } else {
            icons::SPEAKER
        }
    });

    view! {
        row(class = "toolbar", a11y:role = Role::Toolbar) {
            row(class = "tool-group") {
                Tool(svg = icons::CONNECT, label = "Connect", tone = "tone-good", on_press = std::rc::Rc::new(move || state.modal.set(Modal::Connect)))
                Tool(svg = icons::DISCONNECT, label = "Disconnect", tone = "tone-bad", disabled = offline,
                    on_press = std::rc::Rc::new(move || state.disconnect()))
                Tool(svg = icons::BOOKMARK, label = "Connect to the public server", tone = "tone-star",
                    on_press = std::rc::Rc::new(move || {
                        let nickname = state.settings.with_untracked(|s| s.nickname.clone());
                        state.connect(crate::ui::state::DEFAULT_ADDRESS.into(), nickname);
                    }))
            }
            row(class = "tool-group") {
                control(
                    class = "tool",
                    class:danger = move || state.mic_muted.get(),
                    tabindex = Focus::Sequential,
                    a11y:label = "Mute microphone",
                    attr:title = Some("Mute microphone (⌘M)".to_owned()),
                    on:click = move |_| state.set_mic_muted(!state.mic_muted.get_untracked())
                ) { DynIco(svg = mic) }
                control(
                    class = "tool",
                    class:danger = move || state.deafened.get(),
                    tabindex = Focus::Sequential,
                    a11y:label = "Mute speakers",
                    attr:title = Some("Mute speakers".to_owned()),
                    on:click = move |_| state.set_deafened(!state.deafened.get_untracked())
                ) { DynIco(svg = speaker) }
                Tool(svg = icons::AWAY, label = "Set away", active = Signal::derive(move || state.away.get()),
                    on_press = std::rc::Rc::new(move || state.set_away(!state.away.get_untracked())))
            }
            row(class = "tool-group") {
                Tool(svg = icons::ADD_CHANNEL, label = "Create channel", disabled = not_connected,
                    on_press = std::rc::Rc::new(move || state.modal.set(Modal::CreateChannel { parent: None })))
                Tool(svg = icons::SETTINGS, label = "Options", on_press = std::rc::Rc::new(move || state.modal.set(Modal::Options)))
            }
            control(
                class = "share-btn",
                class:live = move || state.sharing.get(),
                class:disabled = move || !state.connected(),
                tabindex = Focus::Sequential,
                a11y:label = "Share screen",
                on:click = move |_| if state.connected() { state.toggle_share() }
            ) {
                DynIco(svg = Signal::derive(move || if state.sharing.get() { icons::SCREEN_OFF } else { icons::SCREEN }))
                text {{move || if state.sharing.get() { "Stop sharing" } else { "Share screen" }}}
            }
            spacer()
            // Push to talk, for when the voice mode asks for it.
            if move || state.voice_mode.get() == VoiceMode::PushToTalk {
                control(
                    class = "ptt",
                    class:down = move || state.ptt.get(),
                    a11y:label = "Push to talk",
                    on:pointer_down = move |_| state.set_ptt(true),
                    on:pointer_up = move |_| state.set_ptt(false),
                    on:pointer_leave = move |_| state.set_ptt(false)
                ) {
                    Ico(svg = icons::MIC)
                    text {{move || if state.ptt.get() { "Talking…" } else { "Hold to talk (F1)" }}}
                }
            }
        }
    }
}
