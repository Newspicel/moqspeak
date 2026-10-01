//! Minimise, maximise and close.

use zgui::prelude::*;

use crate::ui::frame::source::own_frame;
use crate::ui::parts::icons::{self, IconProps, IconSize};
use crate::ui::parts::{Erase, press};

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
