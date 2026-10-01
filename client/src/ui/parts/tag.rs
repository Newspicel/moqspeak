//! A small word beside a name.

use zgui::prelude::*;

/// A tag such as `you`, `mod` or `live`, painted from a tone.
#[component]
pub fn Tag(
    /// The word.
    #[prop(into)]
    text: String,
    /// Which tone paints it: empty for the quiet one, or `primary`, `err`.
    #[prop(default = "")]
    tone: &'static str,
) -> impl IntoView {
    view! {
        text(class = "ms-tag", attr:data-tone = (!tone.is_empty()).then(|| tone.to_owned())) {{text}}
    }
}
