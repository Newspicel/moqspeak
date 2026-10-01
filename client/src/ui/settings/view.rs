//! The settings page: the sections down a column, their controls beside it.

use std::rc::Rc;

use zgui::prelude::*;
use zgui_ui::prelude::*;

use crate::ui::parts::icons::{self, IconProps, IconSize};
use crate::ui::parts::{Erase, KeyProps, press};
use crate::ui::settings::about::AboutPaneProps;
use crate::ui::settings::appearance::AppearancePaneProps;
use crate::ui::settings::sound::SoundPaneProps;
use crate::ui::settings::voice::VoicePaneProps;
use crate::ui::state::AppState;

/// Every section: its pane, its mark and its name.
const SECTIONS: [(&str, &str, &str); 4] = [
    ("voice", icons::MIC, "Voice"),
    ("sound", icons::VOLUME_2, "Sound"),
    ("appearance", icons::PALETTE, "Appearance"),
    ("about", icons::INFO, "About"),
];

/// The page. Escape or the back key returns to the server.
#[component]
pub fn SettingsView() -> impl IntoView {
    let state = AppState::expect();
    let info = RwSignal::new_local(state.engine.with_value(|e| e.audio.device_info()));
    let back = Rc::new(move || state.settings_open.set(false));
    let page = Binding::controlled(
        Signal::derive_local(move || state.settings_page.get()),
        move |next: String| state.settings_page.set(next),
    );
    let entries = SECTIONS
        .into_iter()
        .map(|(value, svg, label)| {
            view! {
                SettingsPage(value = value, on:pointer_down = press::press()) {
                    Icon(svg = svg, size = IconSize::Sm)
                    text {{label.to_owned()}}
                }
            }
            .any()
        })
        .collect::<Vec<_>>();

    view! {
        Settings(page = page, label = "Settings", class = "ms-settings") {
            SettingsPages {
                row(class = "ms-settings__head", on:pointer_down = press::move_window()) {
                    Key(
                        svg = Signal::stored_local(icons::CHEVRON_RIGHT),
                        label = Signal::stored_local("Back".to_owned()),
                        class = "ms-settings__back",
                        on_press = back
                    )
                    text {"Settings"}
                }
                {entries}
            }
            column(class = "ms-settings__main") {
                box(class = "ms-settings__bar", on:pointer_down = press::move_window())
                VoicePane(info = info)
                SoundPane(info = info)
                AppearancePane()
                AboutPane()
            }
        }
    }
}
