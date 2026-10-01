//! A ghost key: an icon that shows a tone only under the pointer, named by a tooltip.

use std::rc::Rc;

use zgui::prelude::*;
use zgui::reactive::{LocalStorage, StoredValue};
use zgui::vocab::SharedString;
use zgui_ui::prelude::*;
use zgui_ui_primitives::Placement;

use crate::ui::parts::icons::{IconProps, IconSize};
use crate::ui::parts::press;

/// An icon key with a tooltip.
///
/// `on` marks a key that holds a state, such as a muted microphone; the sheet paints it in the
/// key's `tone`. `dot` lights a small mark in the corner, such as unread messages.
#[allow(
    clippy::too_many_arguments,
    reason = "each prop is one axis of the key"
)]
#[component]
pub fn Key(
    /// The icon.
    #[prop(into)]
    svg: Signal<&'static str, LocalStorage>,
    /// What the key does, for the tooltip and a reader.
    #[prop(into)]
    label: Signal<String, LocalStorage>,
    /// What happens on a press.
    on_press: Rc<dyn Fn()>,
    /// Whether the key holds its state.
    #[prop(into, default = Signal::stored_local(false))]
    on: Signal<bool, LocalStorage>,
    /// Whether the corner mark shows.
    #[prop(into, default = Signal::stored_local(false))]
    dot: Signal<bool, LocalStorage>,
    /// Whether the key takes no press.
    #[prop(into, default = Signal::stored_local(false))]
    disabled: Signal<bool, LocalStorage>,
    /// The tone a held state is painted in: `err`, `warn` or empty for the accent.
    #[prop(default = "")]
    tone: &'static str,
    /// Where the tooltip stands.
    #[prop(default = Placement::BOTTOM)]
    placement: Placement,
    /// Classes merged after the key's own.
    #[prop(into, optional)]
    class: Classes,
) -> impl IntoView {
    let tip = StoredValue::new_local(label);
    view! {
        Tooltip {
            TooltipTrigger {
                control(
                    class = "ms-key",
                    class = class,
                    attr:data-on = move || on.get().then(|| "true".to_owned()),
                    attr:data-tone = (!tone.is_empty()).then(|| tone.to_owned()),
                    attr:data-disabled = move || disabled.get().then(|| "true".to_owned()),
                    tabindex = Focus::Sequential,
                    a11y:role = Role::Button,
                    a11y:label = move || SharedString::from(label.get()),
                    a11y:disabled = move || disabled.get(),
                    on:pointer_down = press::press(),
                    on:click = move |_| if !disabled.get_untracked() { on_press() }
                ) {
                    Icon(svg = svg, size = IconSize::Sm)
                    if move || dot.get() {
                        box(class = "ms-key__dot", a11y:hidden = true)
                    }
                }
            }
            TooltipContent(placement = Signal::stored_local(placement)) {{move || tip.get_value().get()}}
        }
    }
}
