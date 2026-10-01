//! The presses that move the window, and the ones a control keeps.

use zgui::prelude::*;

/// A press that moves the window. The first button alone, so a press of the second one reaches
/// whatever asks for a menu.
pub fn move_window() -> impl Fn(&mut EventCx<'_, events::PointerDown>) + 'static {
    let drag = try_use_window().map(|window| window.move_drag_handler());
    handler(
        events::POINTER_DOWN,
        move |ev: &mut EventCx<'_, events::PointerDown>| {
            if ev.button != Some(PointerButton::Primary) {
                return;
            }
            if let Some(drag) = drag.as_ref() {
                drag(ev);
            }
        },
    )
}

/// A press a control keeps, so the region around it starts no window drag.
pub fn hold() -> impl Fn(&mut EventCx<'_, events::PointerDown>) + Copy + 'static {
    handler(
        events::POINTER_DOWN,
        |ev: &mut EventCx<'_, events::PointerDown>| {
            if ev.button == Some(PointerButton::Primary) {
                ev.stop_propagation();
            }
        },
    )
}

/// A press that activates a control the moment the first button goes down. The control keeps
/// the press, so the region around it starts no window drag.
pub fn press() -> impl Fn(&mut EventCx<'_, events::PointerDown>) + Copy + 'static {
    handler(
        events::POINTER_DOWN,
        |ev: &mut EventCx<'_, events::PointerDown>| {
            if ev.button == Some(PointerButton::Primary) {
                ev.stop_propagation();
                ev.activate_now();
            }
        },
    )
}
