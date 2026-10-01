//! A device picker. Opening a device can take a moment, so it happens on a worker.

use std::sync::Arc;

use zgui::prelude::*;
use zgui::reactive::{LocalStorage, StoredValue, UnsyncCallback};
use zgui_ui::prelude::*;

use crate::audio::{Audio, DeviceInfo, list_devices};
use crate::ui::state::AppState;

/// The value a device select uses for "follow the system default".
const SYSTEM: &str = "__system__";

/// The input or output device, with the system default first.
#[component]
pub fn DevicePicker(input: bool, info: RwSignal<DeviceInfo, LocalStorage>) -> impl IntoView {
    let state = AppState::expect();
    let lists = list_devices();
    let mut names = if input { lists.inputs } else { lists.outputs };
    // A host can list one device twice. The list is keyed by name, so each name stands once.
    let mut seen = std::collections::HashSet::new();
    names.retain(|name| seen.insert(name.clone()));
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
            SelectTrigger(class = "ms-settings__control") { SelectValue(placeholder = "Choose a device", class = "ms-settings__value-text") }
            SelectContent {
                SelectItem(value = SYSTEM) {{system_label.get_value()}}
                for name in move || names.get_value(), key = |n: &String| n.clone() {
                    SelectItem(value = name.clone()) {{name.clone()}}
                }
            }
        }
    }
}
