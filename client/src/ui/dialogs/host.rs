//! Hosts one dialog, open while the current modal is its own.
//!
//! Every dialog stays mounted and opens through its `open` binding, which follows
//! `AppState::modal`. Its body is rebuilt each time it opens, so its fields start fresh.

use zgui::prelude::*;
use zgui::reactive::{LocalStorage, RenderEffect, UnsyncCallback};
use zgui_ui::prelude::*;

use crate::ui::state::{AppState, Modal};

/// A dialog open while `is` matches the current modal.
#[component]
pub fn ModalHost(is: fn(&Modal) -> bool, children: ChildrenFn) -> impl IntoView {
    let state = AppState::expect();
    let open: RwSignal<bool, LocalStorage> = RwSignal::new_local(false);
    let sync = RenderEffect::new(move |_| {
        let want = state.modal.with(is);
        if open.get_untracked() != want {
            open.set(want);
        }
    });
    on_cleanup_local(move || drop(sync));
    let on_open_change = UnsyncCallback::new(move |next: bool| {
        if !next && state.modal.with_untracked(is) {
            state.modal.set(Modal::None);
        }
    });
    view! {
        Dialog(open = open, on_open_change = on_open_change) {
            DialogContent(class = "ms-dialog") {
                {children.view()}
            }
        }
    }
}

/// A label above a control.
#[component]
pub fn FormRow(#[prop(into)] label: String, children: Children) -> impl IntoView {
    view! {
        column(class = "ms-form__row") {
            text(class = "ms-form__label") {{label}}
            {children.into_view_once()}
        }
    }
}
