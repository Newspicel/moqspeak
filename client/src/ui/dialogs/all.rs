//! Every dialog, each in its host.

use zgui::prelude::*;

use crate::ui::dialogs::connect::ConnectBodyProps;
use crate::ui::dialogs::create_channel::CreateChannelBodyProps;
use crate::ui::dialogs::host::ModalHostProps;
use crate::ui::dialogs::poke::PokeBodyProps;
use crate::ui::screen::ShareDialogProps;
use crate::ui::state::Modal;

/// Every dialog the application has.
#[component]
pub fn Dialogs() -> impl IntoView {
    view! {
        ModalHost(is = |m| matches!(m, Modal::Connect)) { ConnectBody() }
        ModalHost(is = |m| matches!(m, Modal::CreateChannel { .. })) { CreateChannelBody() }
        ModalHost(is = |m| matches!(m, Modal::Poke { .. })) { PokeBody() }
        ShareDialog()
    }
}
