//! One channel of the tree: the fold, the mark, the name and how many are in it.

use zgui::prelude::*;
use zgui::reactive::LocalStorage;
use zgui::vocab::SharedString;
use zgui_ui::prelude::*;

use crate::model::ChannelId;
use crate::ui::parts::icons::{self, IconProps, IconSize};
use crate::ui::parts::press;
use crate::ui::state::{AppState, ChannelRow, Selection};
use crate::ui::tree::channel_menu::ChannelMenuProps;
use crate::ui::tree::view::indent;

/// A channel row. A double press or Enter joins it; the arrows fold it.
#[component]
pub fn ChannelLine(id: ChannelId, row: Signal<ChannelRow, LocalStorage>) -> impl IntoView {
    let state = AppState::expect();
    let field = move |f: fn(&ChannelRow) -> bool| Signal::derive_local(move || row.with(f));
    let collapsed = field(|r| r.collapsed);
    let has_children = field(|r| r.has_children);
    let mark = Signal::derive_local(move || {
        if row.with(|r| r.is_default) {
            icons::HOUSE
        } else {
            icons::HASH
        }
    });

    view! {
        ContextMenu {
            ContextMenuTrigger {
                row(
                    class = "ms-row ms-row--channel",
                    attr:data-selected = move || (state.selected.get() == Selection::Channel(id)).then(|| "true".to_owned()),
                    attr:data-mine = move || row.with(|r| r.mine).then(|| "true".to_owned()),
                    attr:data-drop = move || (state.dragging() && state.drop_target.get() == Some(id)).then(|| "true".to_owned()),
                    style:padding-left = move || indent(row.with(|r| r.depth)),
                    tabindex = Focus::Sequential,
                    a11y:role = Role::TreeItem,
                    a11y:label = move || SharedString::from(row.with(|r| r.name.clone())),
                    a11y:expanded = move || !collapsed.get(),
                    on:click = move |ev| {
                        if ev.button == Some(PointerButton::Secondary) {
                            return;
                        }
                        if state.click(Selection::Channel(id)) {
                            state.join(id);
                        }
                    },
                    on:key_down = move |ev| match &ev.key {
                        Key::Named(NamedKey::Enter) => state.join(id),
                        Key::Named(NamedKey::ArrowLeft) if !collapsed.get_untracked() && has_children.get_untracked() => state.toggle_collapsed(id),
                        Key::Named(NamedKey::ArrowRight) if collapsed.get_untracked() => state.toggle_collapsed(id),
                        _ => {}
                    },
                    on:pointer_enter = move |_| {
                        if state.drag.with_untracked(|d| d.as_ref().is_some_and(|d| d.active)) {
                            state.drop_target.set(Some(id));
                        }
                    },
                    on:pointer_leave = move |_| {
                        if state.drop_target.get_untracked() == Some(id) {
                            state.drop_target.set(None);
                        }
                    }
                ) {
                    control(
                        class = "ms-twisty",
                        attr:data-open = move || (!collapsed.get()).then(|| "true".to_owned()),
                        attr:data-empty = move || (!has_children.get()).then(|| "true".to_owned()),
                        a11y:label = move || SharedString::from(if collapsed.get() { "Expand" } else { "Collapse" }),
                        on:pointer_down = press::hold(),
                        on:click:stop = move |_| if has_children.get_untracked() { state.toggle_collapsed(id) }
                    ) {
                        Icon(svg = icons::CHEVRON_RIGHT, size = IconSize::Xs)
                    }
                    Icon(svg = mark, size = IconSize::Sm, class = "ms-row__mark")
                    text(class = "ms-row__name") {{move || row.with(|r| r.name.clone())}}
                    box(class = "ms-row__gap")
                    if move || row.with(|r| r.count > 0) {
                        text(
                            class = "ms-row__count",
                            attr:data-full = move || row.with(|r| r.full).then(|| "true".to_owned())
                        ) {{move || row.with(|r| r.count.to_string())}}
                    }
                }
            }
            ChannelMenu(id = id, row = row)
        }
    }
}
