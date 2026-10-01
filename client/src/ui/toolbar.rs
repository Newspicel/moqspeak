//! The toolbar under the menu, built from zgui-ui buttons, toggles and tooltips.

use std::time::Duration;

use zgui::prelude::*;
use zgui::reactive::{StoredValue, UnsyncCallback};
use zgui_ui::prelude::*;

use crate::audio::VoiceMode;
use crate::engine::ConnStatus;
use crate::ui::IntoAny;
use crate::ui::icons::{self, DynIcoProps, IcoProps};
use crate::ui::state::{AppState, Modal};

/// An icon button with a tooltip.
#[component]
fn Tool(
    svg: &'static str,
    #[prop(into)] label: String,
    #[prop(default = "")] tone: &'static str,
    #[prop(into, optional)] disabled: Option<Signal<bool>>,
    on_press: std::rc::Rc<dyn Fn()>,
) -> impl IntoView {
    let disabled = disabled.unwrap_or_else(|| Signal::stored(false));
    let tip = StoredValue::new(label.clone());
    view! {
        Tooltip(delay = Duration::from_millis(350)) {
            TooltipTrigger {
                Button(
                    variant = ButtonVariant::Ghost,
                    size = ButtonSize::Icon,
                    disabled = Signal::derive_local(move || disabled.get()),
                    class = format!("tool {tone}"),
                    a11y:label = label.clone(),
                    on:click = move |_| on_press()
                ) { Ico(svg = svg) }
            }
            TooltipContent {{tip.get_value()}}
        }
    }
}

/// A two-state icon button with a tooltip, such as mute.
#[component]
fn ToolToggle(
    on: &'static str,
    off: &'static str,
    #[prop(into)] label: String,
    pressed: Signal<bool>,
    on_change: UnsyncCallback<bool>,
) -> impl IntoView {
    let tip = StoredValue::new(label.clone());
    let icon = Signal::derive(move || if pressed.get() { on } else { off });
    view! {
        Tooltip(delay = Duration::from_millis(350)) {
            TooltipTrigger {
                Toggle(
                    pressed = Binding::controlled(Signal::derive_local(move || pressed.get()), move |v| on_change.run(v)),
                    class = "tool tool-toggle",
                    label = label.clone()
                ) { DynIco(svg = icon) }
            }
            TooltipContent {{tip.get_value()}}
        }
    }
}

#[component]
pub fn Toolbar() -> impl IntoView {
    let state = AppState::expect();
    let offline = Signal::derive(move || {
        state
            .status
            .with(|s| matches!(s, ConnStatus::Disconnected | ConnStatus::Failed(_)))
    });
    let not_connected = Signal::derive(move || !state.connected());

    view! {
        row(class = "toolbar", a11y:role = Role::Toolbar) {
            ButtonGroup(class = "tool-group") {
                Tool(svg = icons::CONNECT, label = "Connect…", tone = "tone-good",
                    on_press = std::rc::Rc::new(move || state.modal.set(Modal::Connect)))
                Tool(svg = icons::DISCONNECT, label = "Disconnect", tone = "tone-bad", disabled = offline,
                    on_press = std::rc::Rc::new(move || state.disconnect()))
                Tool(svg = icons::BOOKMARK, label = "Connect to the public server", tone = "tone-star",
                    on_press = std::rc::Rc::new(move || {
                        let nickname = state.settings.with_untracked(|s| s.nickname.clone());
                        state.connect(crate::ui::state::DEFAULT_ADDRESS.into(), nickname);
                    }))
            }
            Separator(orientation = SeparatorOrientation::Vertical, class = "tool-sep")
            ButtonGroup(class = "tool-group") {
                ToolToggle(on = icons::MIC_MUTED, off = icons::MIC, label = "Mute microphone",
                    pressed = Signal::derive(move || state.mic_muted.get()),
                    on_change = UnsyncCallback::new(move |v: bool| state.set_mic_muted(v)))
                ToolToggle(on = icons::SPEAKER_MUTED, off = icons::SPEAKER, label = "Mute speakers",
                    pressed = Signal::derive(move || state.deafened.get()),
                    on_change = UnsyncCallback::new(move |v: bool| state.set_deafened(v)))
                ToolToggle(on = icons::AWAY, off = icons::AWAY, label = "Away",
                    pressed = Signal::derive(move || state.away.get()),
                    on_change = UnsyncCallback::new(move |v: bool| state.set_away(v)))
            }
            Separator(orientation = SeparatorOrientation::Vertical, class = "tool-sep")
            ButtonGroup(class = "tool-group") {
                Tool(svg = icons::ADD_CHANNEL, label = "Create channel…", disabled = not_connected,
                    on_press = std::rc::Rc::new(move || state.modal.set(Modal::CreateChannel { parent: None })))
                Tool(svg = icons::SETTINGS, label = "Options",
                    on_press = std::rc::Rc::new(move || state.modal.set(Modal::Options)))
            }
            spacer()
            if move || state.voice_mode.get() == VoiceMode::PushToTalk {
                Button(
                    variant = ButtonVariant::Outline,
                    class = "ptt",
                    class:down = move || state.ptt.get(),
                    a11y:label = "Push to talk",
                    on:pointer_down = move |_| state.set_ptt(true),
                    on:pointer_up = move |_| state.set_ptt(false),
                    on:pointer_leave = move |_| state.set_ptt(false)
                ) {
                    Ico(svg = icons::MIC)
                    text {{move || if state.ptt.get() { "Talking…" } else { "Hold to talk" }}}
                    Kbd {"F1"}
                }
            }
            {move || {
                if state.sharing.get() {
                    view! {
                        Button(variant = ButtonVariant::Destructive, on:click = move |_| state.toggle_share()) {
                            Ico(svg = icons::SCREEN_OFF)
                            text {"Stop sharing"}
                        }
                    }
                    .into_any()
                } else {
                    view! {
                        Button(
                            variant = ButtonVariant::Outline,
                            disabled = Signal::derive_local(move || !state.connected()),
                            on:click = move |_| state.toggle_share()
                        ) {
                            Ico(svg = icons::SCREEN)
                            text {"Share screen"}
                        }
                    }
                    .into_any()
                }
            }}
        }
    }
}
