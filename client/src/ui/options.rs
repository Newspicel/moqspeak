//! The Options dialog: capture, playback and appearance.

use std::sync::Arc;
use std::sync::atomic::Ordering;

use zgui::prelude::*;
use zgui::reactive::{LocalStorage, StoredValue, UnsyncCallback};
use zgui_ui::prelude::*;

use crate::audio::{Audio, DeviceInfo, VoiceMode, list_devices};
use crate::ui::state::{AppState, Theme, mode_from_str, mode_to_str};

/// The value a device select uses for "follow the system default".
const SYSTEM: &str = "__system__";

/// A titled group of settings.
#[component]
fn Section(#[prop(into)] title: String, children: Children) -> impl IntoView {
    view! {
        column(class = "opt-section") {
            text(class = "opt-title") {{title}}
            {children.into_view_once()}
        }
    }
}

/// A row with a label on the left and a control on the right.
#[component]
fn Setting(#[prop(into)] label: String, children: Children) -> impl IntoView {
    view! {
        row(class = "opt-row") {
            text(class = "opt-label") {{label}}
            box(class = "opt-control") {{children.into_view_once()}}
        }
    }
}

/// A slider with its value printed beside it.
#[component]
fn ValueSlider(
    value: RwSignal<f64, LocalStorage>,
    min: f64,
    max: f64,
    step: f64,
    #[prop(into)] label: String,
    unit: &'static str,
    on_change: UnsyncCallback<f64>,
) -> impl IntoView {
    view! {
        row(class = "slider-row") {
            Slider(value = value, min = min, max = max, step = step, label = label, class = "slider-grow", on_change = on_change)
            text(class = "slider-value") {{move || format!("{:.0}{unit}", value.get())}}
        }
    }
}

/// A device picker. Opening a device can take a moment, so it happens on a worker.
#[component]
fn DevicePicker(input: bool, info: RwSignal<DeviceInfo, LocalStorage>) -> impl IntoView {
    let state = AppState::expect();
    let lists = list_devices();
    let names = if input { lists.inputs } else { lists.outputs };
    let default = if input {
        lists.default_input
    } else {
        lists.default_output
    };
    let saved = state.settings.with_untracked(|s| {
        if input {
            s.input_device.clone()
        } else {
            s.output_device.clone()
        }
    });
    let value = RwSignal::new_local(saved.unwrap_or_else(|| SYSTEM.to_owned()));
    let audio: StoredValue<Arc<Audio>> =
        StoredValue::new(state.engine.with_value(|e| e.audio.clone()));
    let system_label = StoredValue::new(format!(
        "System default{}",
        default.map(|d| format!(" ({d})")).unwrap_or_default()
    ));
    let names = StoredValue::new(names);

    let on_change = UnsyncCallback::new(move |chosen: String| {
        let name = (chosen != SYSTEM).then_some(chosen);
        state.update_settings(|s| {
            if input {
                s.input_device = name.clone();
            } else {
                s.output_device = name.clone();
            }
        });
        let audio = audio.get_value();
        spawn_local(async move {
            let next = blocking(move || {
                if input {
                    audio.set_input(name)
                } else {
                    audio.set_output(name)
                }
            })
            .await;
            info.set(next);
        });
    });

    view! {
        Select(value = value, on_change = on_change) {
            SelectTrigger(class = "device-trigger") { SelectValue(placeholder = "Choose a device") }
            SelectContent {
                SelectItem(value = SYSTEM) {{system_label.get_value()}}
                for name in move || names.get_value(), key = |n: &String| n.clone() {
                    SelectItem(value = name.clone()) {{name.clone()}}
                }
            }
        }
    }
}

#[component]
pub fn OptionsBody() -> impl IntoView {
    let state = AppState::expect();
    let settings = state.settings.get_untracked();
    let threshold = RwSignal::new_local(settings.threshold_db as f64);
    let gain = RwSignal::new_local((settings.input_gain * 100.0) as f64);
    let output = RwSignal::new_local((settings.output_volume * 100.0) as f64);
    let info = RwSignal::new_local(state.engine.with_value(|e| e.audio.device_info()));
    let shared = state.engine.with_value(|e| e.audio.shared.clone());

    // Poll the microphone level for the meter while the dialog is open.
    let level = RwSignal::new_local(-100.0f32);
    let speaking = RwSignal::new_local(false);
    let voice = RwSignal::new_local(0.0f32);
    let timer = {
        let shared = shared.clone();
        set_interval(std::time::Duration::from_millis(50), move || {
            level.set(shared.level_db.get());
            speaking.set(shared.transmitting.load(Ordering::Relaxed));
            voice.set(shared.voice_prob.get());
        })
    };
    on_cleanup_local(move || drop(timer));

    let loopback = RwSignal::new_local(shared.loopback.load(Ordering::Relaxed));
    let mode = Binding::controlled(
        Signal::derive_local(move || vec![mode_to_str(state.voice_mode.get()).to_owned()]),
        move |v: Vec<String>| {
            if let Some(m) = v.first() {
                state.set_voice_mode(mode_from_str(m));
            }
        },
    );
    let theme = Binding::controlled(
        Signal::derive_local(move || vec![state.theme.get().name().to_owned()]),
        move |v: Vec<String>| {
            if let Some(t) = v.first() {
                state.set_theme(Theme::parse(t));
            }
        },
    );
    let page = RwSignal::new_local("capture".to_owned());
    let pct = |db: f32| ((db + 80.0) / 80.0 * 100.0).clamp(0.0, 100.0);

    view! {
                DialogHeader {
                    DialogTitle {"Options"}
                }
                // The strip is zgui-ui's; the pages are plain branches, because switching a
                // TabsContent inside a Dialog drops the dialog's centring (zortax/zgui#2).
                Tabs(value = page, label = "Options", class = "opt-tabs") {
                    TabsList {
                        TabsTrigger(value = "capture") {"Capture"}
                        TabsTrigger(value = "playback") {"Playback"}
                        TabsTrigger(value = "appearance") {"Appearance"}
                    }
                }
                    if move || page.get() == "capture" {
                        column(class = "opt-page") {
                            Section(title = "Microphone") {
                                Setting(label = "Device") { DevicePicker(input = true, info = info) }
                                Setting(label = "Gain") {
                                    ValueSlider(value = gain, min = 0.0, max = 400.0, step = 5.0, label = "Microphone gain", unit = "%",
                                        on_change = UnsyncCallback::new(move |v: f64| {
                                            state.engine.with_value(|e| e.audio.shared.input_gain.set((v / 100.0) as f32));
                                            state.update_settings(|s| s.input_gain = (v / 100.0) as f32);
                                        }))
                                }
                            }
                            Section(title = "Processing") {
                                Setting(label = "Noise suppression") {
                                    row(class = "switch-row") {
                                        Switch(
                                            default_checked = settings.noise_suppression,
                                            on_change = UnsyncCallback::new(move |on: bool| {
                                                state.engine.with_value(|e| e.audio.shared.noise_suppression.store(on, Ordering::Relaxed));
                                                state.update_settings(|s| s.noise_suppression = on);
                                            })
                                        )
                                        text(class = "opt-hint") {"RNNoise removes fans, keyboards and hum."}
                                    }
                                }
                                Setting(label = "Speech detection") {
                                    row(class = "switch-row") {
                                        Switch(
                                            default_checked = settings.smart_vad,
                                            on_change = UnsyncCallback::new(move |on: bool| {
                                                state.engine.with_value(|e| e.audio.shared.smart_vad.store(on, Ordering::Relaxed));
                                                state.update_settings(|s| s.smart_vad = on);
                                            })
                                        )
                                        text(class = "opt-hint") {"A neural detector (earshot) opens the mic only for voice."}
                                    }
                                }
                            }
                            Section(title = "Transmission") {
                                ToggleGroup(value = mode, variant = ToggleVariant::Outline, label = "Transmission", class = "picker") {
                                    ToggleGroupItem(value = "activation") {"Voice activation"}
                                    ToggleGroupItem(value = "ptt") {"Push-to-talk"}
                                    ToggleGroupItem(value = "continuous") {"Continuous"}
                                }
                                if move || state.voice_mode.get() == VoiceMode::PushToTalk {
                                    text(class = "opt-hint") {"Hold F1 anywhere, ` outside text fields, or the toolbar button."}
                                }
                                if move || state.voice_mode.get() == VoiceMode::Activation {
                                    column(class = "vad") {
                                        row(class = "meter", class:speaking = move || speaking.get()) {
                                            box(class = "meter-fill", class:hot = move || level.get() >= threshold.get() as f32,
                                                style:width = move || Some(format!("{:.1}%", pct(level.get())))) {}
                                            box(class = "meter-mark", style:left = move || Some(format!("{:.1}%", pct(threshold.get() as f32)))) {}
                                        }
                                        ValueSlider(value = threshold, min = -80.0, max = 0.0, step = 1.0, label = "Activation threshold", unit = " dB",
                                            on_change = UnsyncCallback::new(move |v: f64| {
                                                state.engine.with_value(|e| e.audio.shared.threshold_db.set(v as f32));
                                                state.update_settings(|s| s.threshold_db = v as f32);
                                            }))
                                        row(class = "vad-readout") {
                                            text(class = "opt-hint") {"Speak normally: the bar turns green above the red marker."}
                                            spacer()
                                            text(class = "vad-prob", class:on = move || voice.get() >= 0.5) {{move || format!("speech {:.0}%", voice.get() * 100.0)}}
                                        }
                                    }
                                }
                            }
                            Section(title = "Test") {
                                Button(
                                    variant = ButtonVariant::Outline,
                                    on:click = move |_| {
                                        let on = !loopback.get_untracked();
                                        loopback.set(on);
                                        state.engine.with_value(|e| e.audio.shared.loopback.store(on, Ordering::Relaxed));
                                    }
                                ) {{move || if loopback.get() { "Stop hearing myself" } else { "Hear myself" }}}
                            }
                        }
                    }
                    if move || page.get() == "playback" {
                        column(class = "opt-page") {
                            Section(title = "Speakers") {
                                Setting(label = "Device") { DevicePicker(input = false, info = info) }
                                Setting(label = "Volume") {
                                    ValueSlider(value = output, min = 0.0, max = 200.0, step = 5.0, label = "Output volume", unit = "%",
                                        on_change = UnsyncCallback::new(move |v: f64| {
                                            state.engine.with_value(|e| e.audio.shared.master_volume.set((v / 100.0) as f32));
                                            state.update_settings(|s| s.output_volume = (v / 100.0) as f32);
                                        }))
                                }
                            }
                        }
                    }
                    if move || page.get() == "appearance" {
                        column(class = "opt-page") {
                            Section(title = "Theme") {
                                ToggleGroup(value = theme, variant = ToggleVariant::Outline, label = "Theme", class = "picker") {
                                    ToggleGroupItem(value = "system") {"System"}
                                    ToggleGroupItem(value = "dark") {"Dark"}
                                    ToggleGroupItem(value = "light") {"Light"}
                                }
                            }
                        }
                    }
                column(class = "opt-status") {
                    text {{move || format!("Microphone: {}", info.get().input.unwrap_or_else(|| "none".into()))}}
                    text {{move || format!("Speakers: {}", info.get().output.unwrap_or_else(|| "none".into()))}}
                    if move || info.with(|i| i.error.is_some()) {
                        text(class = "form-error") {{move || info.get().error.unwrap_or_default()}}
                    }
                }
                DialogFooter { DialogClose {"Done"} }
    }
}
