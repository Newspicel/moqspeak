//! Minimise, maximise and close.

use zgui::prelude::*;

use crate::ui::parts::icons::{self, IconProps, IconSize};
use crate::ui::parts::{Erase, press};

/// Whether the desktop draws the frame around the window.
///
/// macOS keeps its frame for a window that asks for no title bar, so the window buttons, the
/// corners and the resize edges belong to the system. The other desktops hand the whole frame
/// over, and the window draws them itself.
pub const PLATFORM_FRAME: bool = cfg!(target_os = "macos");

/// Draws `view` where the desktop leaves it to the window.
pub fn own_frame(view: impl IntoView + 'static) -> AnyView {
    if PLATFORM_FRAME { ().any() } else { view.any() }
}

/// One window key.
fn key(
    label: &'static str,
    class: &'static str,
    svg: Signal<&'static str, zgui::reactive::LocalStorage>,
    run: impl Fn() + 'static,
) -> AnyView {
    view! {
        control(
            class = class,
            on:pointer_down = press::hold(),
            tabindex = Focus::Sequential,
            a11y:role = Role::Button,
            a11y:label = label,
            on:click = move |_| run()
        ) {
            Icon(svg = svg, size = IconSize::Xs)
        }
    }
    .any()
}

/// The three window keys at the top right corner. A press on the corner itself moves the window.
#[component]
pub fn WindowControls() -> impl IntoView {
    let Some(window) = try_use_window() else {
        return ().any();
    };
    let maximized = window.maximized();
    let (min, max, close) = (window.clone(), window.clone(), window);
    let shape = Signal::derive_local(move || {
        if maximized.get() {
            icons::COPY
        } else {
            icons::SQUARE
        }
    });

    own_frame(view! {
        row(class = "ms-corner", on:pointer_down = press::move_window()) {
            {key("Minimise", "ms-corner__key", Signal::stored_local(icons::MINUS), move || min.minimize())}
            {key("Maximise", "ms-corner__key", shape, move || max.toggle_maximized())}
            {key("Close", "ms-corner__key ms-corner__key--close", Signal::stored_local(icons::X), move || close.close())}
        }
    })
}
