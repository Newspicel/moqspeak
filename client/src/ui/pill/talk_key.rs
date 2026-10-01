//! The key held to talk, for push-to-talk without the keyboard.

use zgui::prelude::*;
use zgui_ui::prelude::*;
use zgui_ui_primitives::Placement;

use crate::ui::parts::icons::{self, IconProps, IconSize};
use crate::ui::state::AppState;

/// Transmits while the first button holds it down.
#[component]
pub fn TalkKey() -> impl IntoView {
    let state = AppState::expect();
    view! {
        Tooltip {
            TooltipTrigger {
                control(
                    class = "ms-key ms-talk",
                    attr:data-on = move || state.ptt.get().then(|| "true".to_owned()),
                    a11y:role = Role::Button,
                    a11y:label = "Hold to talk",
                    on:pointer_down = move |ev| {
                        if ev.button == Some(PointerButton::Primary) {
                            ev.stop_propagation();
                            state.set_ptt(true);
                        }
                    },
                    on:pointer_up = move |_| state.set_ptt(false),
                    on:pointer_leave = move |_| state.set_ptt(false)
                ) {
                    Icon(svg = icons::AUDIO_LINES, size = IconSize::Sm)
                }
            }
            TooltipContent(placement = Signal::stored_local(Placement::TOP)) {"Hold to talk (F1)"}
        }
    }
}
