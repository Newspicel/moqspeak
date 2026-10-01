//! The row the conversation tabs stand in. It scrolls sideways under the wheel, shows no bar, and
//! fades its content out at an edge where more of it waits.

use zgui::geom::{CssPx, DevicePx, Point};
use zgui::prelude::*;
use zgui::view::{ScrollBehavior, ScrollTarget};

/// How tall one line of a notched wheel counts, in CSS pixels.
const LINE: f32 = 16.0;

/// The sideways distance a wheel turn asks for: either axis moves the row.
fn travel(delta: ScrollDelta) -> f32 {
    let pixels = delta.to_pixels(CssPx(LINE));
    pixels.width.0 + pixels.height.0
}

/// The scrolling row. The bar the engine draws stands under the tabs, where the frame clips it.
#[component]
pub fn TabStrip(children: Children) -> impl IntoView {
    let port = NodeRef::new();
    let seen = port.observe_scroll();
    let before = move || seen.with(|p| p.offset.x.0 > 0.5);
    let after = move || seen.with(|p| !p.is_at_end_horizontally());

    view! {
        box(class = "ms-strip") {
            row(
                class = "ms-strip__scroll",
                node_ref = port,
                a11y:role = Role::TabList,
                a11y:label = "Conversations",
                on:wheel:prevent = move |ev| {
                    let by = travel(ev.delta);
                    if by.abs() < f32::EPSILON {
                        return;
                    }
                    // A notched wheel glides; a trackpad follows the fingers.
                    let behavior = if matches!(ev.delta, ScrollDelta::Lines { .. }) {
                        ScrollBehavior::Smooth
                    } else {
                        ScrollBehavior::Instant
                    };
                    port.scroll_to(ScrollTarget::By(Point::new(DevicePx(by), DevicePx(0.0))), behavior);
                }
            ) {
                {children.into_view_once()}
            }
            box(class = "ms-strip__fade", attr:data-edge = "start", attr:data-shown = move || before().then(|| "true".to_owned()), a11y:hidden = true)
            box(class = "ms-strip__fade", attr:data-edge = "end", attr:data-shown = move || after().then(|| "true".to_owned()), a11y:hidden = true)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{LINE, travel};
    use zgui::geom::{CssPx, Size};
    use zgui::prelude::ScrollDelta;

    #[test]
    fn a_wheel_turn_moves_the_row_along_either_axis() {
        assert_eq!(travel(ScrollDelta::Lines { x: 0.0, y: 3.0 }), 3.0 * LINE);
        assert_eq!(
            travel(ScrollDelta::Pixels(Size::new(CssPx(-12.0), CssPx(0.0)))),
            -12.0
        );
    }
}
