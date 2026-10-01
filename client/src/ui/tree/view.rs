//! The channel tree: every channel and who is in it, one row each.

use zgui::prelude::*;
use zgui_ui::prelude::*;

use crate::ui::parts::Erase;
use crate::ui::state::{AppState, Row, RowKey, tree_rows};
use crate::ui::tree::channel_row::ChannelLineProps;
use crate::ui::tree::client_row::ClientLineProps;

/// How far one level of the tree steps in, in CSS pixels. `--ms-indent` holds the same length.
pub const INDENT: u32 = 22;

/// The room before a row's first mark at `depth`.
pub fn indent(depth: u16) -> Option<String> {
    Some(format!(
        "{}px",
        8 + u32::from(depth.saturating_sub(1)) * INDENT
    ))
}

/// The tree. Each row keeps its element while the server changes, keyed by what it shows.
#[component]
pub fn Tree() -> impl IntoView {
    let state = AppState::expect();
    let rows = Memo::new(move |_| {
        let me = state.me.get();
        let collapsed = state.collapsed.get();
        state.channels.with(|channels| {
            state
                .clients
                .with(|clients| tree_rows(channels, clients, me, &collapsed))
        })
    });
    let keys = Memo::new(move |_| rows.with(|rows| rows.iter().map(Row::key).collect::<Vec<_>>()));

    view! {
        ScrollArea(class = "ms-tree", label = "Channels") {
            column(class = "ms-tree__rows", a11y:role = Role::Tree, a11y:label = "Channels") {
                for key in move || keys.get(), key = |key: &RowKey| *key {
                    {line(rows, key)}
                }
            }
        }
    }
}

/// The row for `key`, reading its fields from `rows` as they change.
fn line(rows: Memo<Vec<Row>>, key: RowKey) -> AnyView {
    match key {
        RowKey::Channel(id) => {
            let row = Signal::derive_local(move || {
                rows.with(|rows| {
                    rows.iter().find_map(|r| match r {
                        Row::Channel(c) if c.id == id => Some(c.clone()),
                        _ => None,
                    })
                })
                .unwrap_or_default()
            });
            view! { ChannelLine(id = id, row = row) }.any()
        }
        RowKey::Client(id) => {
            let row = Signal::derive_local(move || {
                rows.with(|rows| {
                    rows.iter().find_map(|r| match r {
                        Row::Client(c) if c.id == id => Some(c.clone()),
                        _ => None,
                    })
                })
                .unwrap_or_default()
            });
            view! { ClientLine(id = id, row = row) }.any()
        }
    }
}
