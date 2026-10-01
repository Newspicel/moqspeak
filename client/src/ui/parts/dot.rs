//! The mark that says how something is.

use zgui::prelude::*;
use zgui::reactive::LocalStorage;

/// A small round mark painted from a tone token.
///
/// The tone is one of `ok`, `warn`, `err`, `info`, `idle` and `talk`. The sheet turns it into a
/// colour, so no Rust here knows one. Text beside the mark carries the meaning, so the mark
/// itself stays out of the accessibility tree.
#[component]
pub fn StatusDot(
    /// Which tone paints it.
    #[prop(into)]
    tone: Signal<&'static str, LocalStorage>,
    /// Whether the thing behind the mark is on its way to a state, which makes it pulse.
    #[prop(into, default = Signal::stored_local(false))]
    busy: Signal<bool, LocalStorage>,
) -> impl IntoView {
    view! {
        box(
            class = "ms-dot",
            attr:data-tone = move || Some(tone.get().to_owned()),
            attr:data-busy = move || busy.get().then(|| "true".to_owned()),
            a11y:hidden = true
        )
    }
}
