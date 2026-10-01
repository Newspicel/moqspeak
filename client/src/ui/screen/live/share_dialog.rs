//! The display picker in its dialog host.

use zgui::prelude::*;

use crate::ui::dialogs::ModalHostProps;
use crate::ui::screen::live::picker::ShareBodyProps;
use crate::ui::state::Modal;

/// The display picker, open while the modal asks for it.
#[component]
pub fn ShareDialog() -> impl IntoView {
    view! {
        ModalHost(is = |m| matches!(m, Modal::Share { .. })) { ShareBody() }
    }
}
