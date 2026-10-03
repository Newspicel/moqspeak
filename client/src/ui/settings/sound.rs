//! The speakers: the device, the volume and the interface sounds.

use std::cell::Cell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use zgui::prelude::*;
use zgui::reactive::{LocalStorage, UnsyncCallback};
use zgui_ui::prelude::*;

use crate::audio::{Cue, DeviceInfo};
use crate::ui::settings::device::DevicePickerProps;
use crate::ui::settings::slider::ValueSliderProps;
use crate::ui::state::AppState;

/// The shortest time between two previews while the sound volume slider moves.
const PREVIEW_GAP: Duration = Duration::from_millis(250);

/// The sound pane.
#[component]
pub fn SoundPane(info: RwSignal<DeviceInfo, LocalStorage>) -> impl IntoView {
    let state = AppState::expect();
    let settings = state.settings.get_untracked();
    let volume = RwSignal::new_local(f64::from(settings.output_volume) * 100.0);
    let sound_volume = RwSignal::new_local(f64::from(settings.sound_volume) * 100.0);
    let last_preview = Rc::new(Cell::new(None::<Instant>));
    let sounds = UnsyncCallback::new(move |on: bool| {
        state.update_settings(|s| s.sounds = on);
        state.cue(Cue::Unmute);
    });
    let sound_volume_changed = UnsyncCallback::new(move |v: f64| {
        state
            .engine
            .with_value(|e| e.audio.shared.cue_volume.set((v / 100.0) as f32));
        state.update_settings(|s| s.sound_volume = (v / 100.0) as f32);
        let now = Instant::now();
        if last_preview.get().is_none_or(|at| now - at >= PREVIEW_GAP) {
            last_preview.set(Some(now));
            state.cue(Cue::PeerJoin);
        }
    });
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
            SettingsGroup {
                SettingsGroupLabel {"Interface sounds"}
                SettingsItem(label = "Play sounds", description = "Mute, deafen, moves, pokes and messages.") {
                    Switch(default_checked = settings.sounds, on_change = sounds, {..use_settings_item_attrs()})
                }
                if move || state.settings.with(|s| s.sounds) {
                    SettingsItem(label = "Volume") {
                        ValueSlider(
                            value = sound_volume, min = 0.0, max = 100.0, step = 5.0, label = "Sound volume", unit = "%",
                            on_change = sound_volume_changed
                        )
                    }
                }
            }
        }
    }
}
