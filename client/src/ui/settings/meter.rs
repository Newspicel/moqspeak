//! The microphone level against the activation threshold, and how sure the detector is that it
//! hears speech.

use std::sync::atomic::Ordering;

use zgui::prelude::*;
use zgui::reactive::{LocalStorage, UnsyncCallback};

use crate::ui::settings::slider::ValueSliderProps;
use crate::ui::state::AppState;

/// Where `db` stands on the meter, from 0 to 100.
fn percent(db: f32) -> f32 {
    ((db + 80.0) / 80.0 * 100.0).clamp(0.0, 100.0)
}

/// The meter, the threshold slider and the speech reading. The engine's readings are sampled
/// while the meter stands.
#[component]
pub fn LevelMeter(threshold: RwSignal<f64, LocalStorage>) -> impl IntoView {
    let state = AppState::expect();
    let shared = state.engine.with_value(|e| e.audio.shared.clone());
    let level = RwSignal::new_local(-100.0f32);
    let voice = RwSignal::new_local(0.0f32);
    let speaking = RwSignal::new_local(false);
    let sampling = set_interval(std::time::Duration::from_millis(50), move || {
        level.set(shared.level_db.get());
        voice.set(shared.voice_prob.get());
        speaking.set(shared.transmitting.load(Ordering::Relaxed));
    });
    on_cleanup_local(move || drop(sampling));

    view! {
        column(class = "ms-vad") {
            box(class = "ms-meter") {
                box(
                    class = "ms-meter__fill",
                    attr:data-hot = move || (level.get() >= threshold.get() as f32).then(|| "true".to_owned()),
                    style:width = move || Some(format!("{:.1}%", percent(level.get())))
                )
                box(class = "ms-meter__mark", style:left = move || Some(format!("{:.1}%", percent(threshold.get() as f32))))
            }
            ValueSlider(
                value = threshold, min = -80.0, max = 0.0, step = 1.0, label = "Activation threshold", unit = " dB",
                on_change = UnsyncCallback::new(move |v: f64| {
                    state.engine.with_value(|e| e.audio.shared.threshold_db.set(v as f32));
                    state.update_settings(|s| s.threshold_db = v as f32);
                })
            )
            row(class = "ms-vad__readout") {
                text {{move || if speaking.get() { "Transmitting" } else { "Quiet" }}}
                text(class = "ms-vad__speech", attr:data-on = move || (voice.get() >= 0.5).then(|| "true".to_owned())) {
                    {move || format!("speech {:.0}%", voice.get() * 100.0)}
                }
            }
        }
    }
}
