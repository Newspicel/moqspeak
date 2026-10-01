//! A slider with its value printed beside it.

use zgui::prelude::*;
use zgui::reactive::{LocalStorage, UnsyncCallback};
use zgui_ui::prelude::*;

/// A slider in the settings column, with its reading at the trailing edge.
#[component]
pub fn ValueSlider(
    value: RwSignal<f64, LocalStorage>,
    min: f64,
    max: f64,
    step: f64,
    #[prop(into)] label: String,
    unit: &'static str,
    on_change: UnsyncCallback<f64>,
) -> impl IntoView {
    view! {
        row(class = "ms-settings__slider") {
            Slider(value = value, min = min, max = max, step = step, label = label, on_change = on_change)
            text(class = "ms-settings__value") {{move || format!("{:.0}{unit}", value.get())}}
        }
    }
}
