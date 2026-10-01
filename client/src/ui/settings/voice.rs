//! The microphone: the device, its gain, how it transmits and what cleans it up.

use std::sync::atomic::Ordering;

use zgui::prelude::*;
use zgui::reactive::{LocalStorage, UnsyncCallback};
use zgui_ui::prelude::*;

use crate::audio::{DeviceInfo, VoiceMode};
use crate::ui::settings::device::DevicePickerProps;
use crate::ui::settings::meter::LevelMeterProps;
use crate::ui::settings::slider::ValueSliderProps;
use crate::ui::state::{AppState, mode_from_str, mode_to_str};

/// One processing switch, written to the engine and the settings file.
fn processing(
    state: AppState,
    write: fn(&crate::audio::AudioShared, bool),
    save: fn(&mut crate::ui::state::Settings, bool),
) -> UnsyncCallback<bool> {
    UnsyncCallback::new(move |on: bool| {
        state.engine.with_value(|e| write(&e.audio.shared, on));
        state.update_settings(|s| save(s, on));
    })
}

/// The voice pane.
#[component]
pub fn VoicePane(info: RwSignal<DeviceInfo, LocalStorage>) -> impl IntoView {
    let state = AppState::expect();
    let settings = state.settings.get_untracked();
    let gain = RwSignal::new_local(f64::from(settings.input_gain) * 100.0);
    let threshold = RwSignal::new_local(f64::from(settings.threshold_db));
    let mode = Binding::controlled(
        Signal::derive_local(move || vec![mode_to_str(state.voice_mode.get()).to_owned()]),
        move |v: Vec<String>| {
            if let Some(m) = v.first() {
                state.set_voice_mode(mode_from_str(m));
            }
        },
    );
    let loopback = Binding::controlled(
        Signal::derive_local(move || state.loopback.get()),
        move |on: bool| state.set_loopback(on),
    );

    view! {
        SettingsPane(value = "voice") {
            SettingsGroup {
                SettingsGroupLabel {"Microphone"}
                SettingsItem(label = "Device") { DevicePicker(input = true, info = info) }
                SettingsItem(label = "Gain") {
                    ValueSlider(
                        value = gain, min = 0.0, max = 400.0, step = 5.0, label = "Microphone gain", unit = "%",
                        on_change = UnsyncCallback::new(move |v: f64| {
                            state.engine.with_value(|e| e.audio.shared.input_gain.set((v / 100.0) as f32));
                            state.update_settings(|s| s.input_gain = (v / 100.0) as f32);
                        })
                    )
                }
                SettingsItem(label = "Hear myself", description = "Plays the microphone back to you.") {
                    Switch(checked = loopback, {..use_settings_item_attrs()})
                }
            }
            SettingsGroup {
                SettingsGroupLabel {"Transmission"}
                SettingsItem(label = "Mode") {
                    ToggleGroup(value = mode, spacing = 2.0, label = "Transmission", class = "ms-switch") {
                        ToggleGroupItem(value = "activation") {"Voice"}
                        ToggleGroupItem(value = "ptt") {"Push-to-talk"}
                        ToggleGroupItem(value = "continuous") {"Continuous"}
                    }
                }
                if move || state.voice_mode.get() == VoiceMode::PushToTalk {
                    text(class = "ms-settings__note") {"Hold F1 anywhere, ` outside text fields, or the talk key in the pill."}
                }
                if move || state.voice_mode.get() == VoiceMode::Activation {
                    SettingsItem(label = "Threshold", description = "Speak normally. The bar lights above the mark.") {
                        LevelMeter(threshold = threshold)
                    }
                }
            }
            SettingsGroup {
                SettingsGroupLabel {"Processing"}
                SettingsItem(label = "Echo cancellation", description = "Keeps your speakers out of your microphone.") {
                    Switch(
                        default_checked = settings.echo_cancellation,
                        on_change = processing(state, |a, on| a.echo_cancellation.store(on, Ordering::Relaxed), |s, on| s.echo_cancellation = on),
                        {..use_settings_item_attrs()}
                    )
                }
                SettingsItem(label = "Noise suppression", description = "Removes fans, keyboards and hum.") {
                    Switch(
                        default_checked = settings.noise_suppression,
                        on_change = processing(state, |a, on| a.noise_suppression.store(on, Ordering::Relaxed), |s, on| s.noise_suppression = on),
                        {..use_settings_item_attrs()}
                    )
                }
                SettingsItem(label = "Speech detection", description = "Opens the microphone for voice alone.") {
                    Switch(
                        default_checked = settings.smart_vad,
                        on_change = processing(state, |a, on| a.smart_vad.store(on, Ordering::Relaxed), |s, on| s.smart_vad = on),
                        {..use_settings_item_attrs()}
                    )
                }
            }
            if move || info.with(|i| i.error.is_some()) {
                text(class = "ms-settings__note", attr:data-tone = "err") {{move || info.get().error.unwrap_or_default()}}
            }
        }
    }
}
