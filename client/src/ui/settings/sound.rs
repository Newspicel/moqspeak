//! The speakers: the device and the volume.

use zgui::prelude::*;
use zgui::reactive::{LocalStorage, UnsyncCallback};
use zgui_ui::prelude::*;

use crate::audio::DeviceInfo;
use crate::ui::settings::device::DevicePickerProps;
use crate::ui::settings::slider::ValueSliderProps;
use crate::ui::state::AppState;

/// The sound pane.
#[component]
pub fn SoundPane(info: RwSignal<DeviceInfo, LocalStorage>) -> impl IntoView {
    let state = AppState::expect();
    let volume =
        RwSignal::new_local(f64::from(state.settings.with_untracked(|s| s.output_volume)) * 100.0);
    view! {
        SettingsPane(value = "sound") {
            SettingsGroup {
                SettingsGroupLabel {"Speakers"}
                SettingsItem(label = "Device") { DevicePicker(input = false, info = info) }
                SettingsItem(label = "Volume") {
                    ValueSlider(
                        value = volume, min = 0.0, max = 200.0, step = 5.0, label = "Output volume", unit = "%",
                        on_change = UnsyncCallback::new(move |v: f64| {
                            state.engine.with_value(|e| e.audio.shared.master_volume.set((v / 100.0) as f32));
                            state.update_settings(|s| s.output_volume = (v / 100.0) as f32);
                        })
                    )
                }
            }
        }
    }
}
