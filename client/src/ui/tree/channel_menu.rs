//! What a channel's menu offers: joining, folding, and for moderators creating and deleting.

use zgui::prelude::*;
use zgui::reactive::LocalStorage;
use zgui_ui::prelude::*;

use crate::model::{ChannelId, ClientMsg, Role};
use crate::ui::parts::act;
use crate::ui::parts::icons::{self, IconProps, IconSize};
use crate::ui::state::{AppState, ChannelRow, Modal};

/// The context menu of one channel.
#[component]
pub fn ChannelMenu(id: ChannelId, row: Signal<ChannelRow, LocalStorage>) -> impl IntoView {
    let state = AppState::expect();
    view! {
        ContextMenuContent {
            MenuItem(disabled = row.with_untracked(|r| r.mine), on_select = act(move || state.join(id))) {
                Icon(svg = icons::LOG_IN, size = IconSize::Sm)
                text {"Join channel"}
            }
            if move || row.with(|r| r.has_children) {
                MenuItem(on_select = act(move || state.toggle_collapsed(id))) {
                    Icon(svg = Signal::derive_local(move || if row.with(|r| r.collapsed) { icons::CHEVRONS_UP_DOWN } else { icons::CHEVRONS_DOWN_UP }), size = IconSize::Sm)
                    text {{move || if row.with(|r| r.collapsed) { "Expand" } else { "Collapse" }}}
                }
            }
            if move || state.my_role() >= Role::Mod {
                MenuSeparator()
                MenuItem(on_select = act(move || state.modal.set(Modal::CreateChannel { parent: Some(id) }))) {
                    Icon(svg = icons::FOLDER_PLUS, size = IconSize::Sm)
                    text {"New sub-channel…"}
                }
            }
            if move || state.my_role() >= Role::Admin {
                MenuItem(
                    destructive = true,
                    disabled = row.with_untracked(|r| r.is_default),
                    on_select = act(move || state.msg(ClientMsg::DeleteChannel { id }))
                ) {
                    Icon(svg = icons::TRASH_2, size = IconSize::Sm)
                    text {"Delete channel"}
                }
            }
        }
    }
}
