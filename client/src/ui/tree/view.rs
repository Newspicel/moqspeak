//! The channel tree: every channel and who is in it, one row each.
//!
//! Every row has one height, so a row stands at its place times that height. Each row keeps its
//! element while the server changes and moves to its new place on the quick clock: a client who
//! changes channel glides there, and the rows around the channels that shrink or grow glide with
//! it.

use zgui::prelude::*;
use zgui_ui::prelude::*;

use crate::ui::parts::Erase;
use crate::ui::state::{AppState, Row, RowKey, tree_rows};
use crate::ui::tree::channel_row::ChannelLineProps;
use crate::ui::tree::client_row::ClientLineProps;

/// How tall one row is, in CSS pixels. `--ms-row` holds the same length.
pub const ROW: f32 = 28.0;

/// How far one level of the tree steps in, in CSS pixels.
pub const INDENT: u32 = 22;

/// How much further a client stands in than a channel at its depth, so it reads as inside the
/// channel above it.
pub const CLIENT_INSET: u32 = 12;

/// The room before a channel's first mark at `depth`.
pub fn indent(depth: u16) -> u32 {
    8 + u32::from(depth.saturating_sub(1)) * INDENT
}

/// A length in CSS pixels, as a style value.
pub fn px(length: u32) -> Option<String> {
    Some(format!("{length}px"))
}

/// Where the row at `place` stands.
fn offset(place: usize) -> Option<String> {
    Some(format!("translate(0px, {}px)", place as f32 * ROW))
}

/// The tree.
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
    let height = move || Some(format!("{}px", keys.with(Vec::len) as f32 * ROW));

    view! {
        ScrollArea(class = "ms-tree", label = "Channels") {
            column(class = "ms-tree__rows") {
                box(
                    class = "ms-tree__field",
                    style:height = height,
                    a11y:role = Role::Tree,
                    a11y:label = "Channels"
                ) {
                    for key in move || keys.get(), key = |key: &RowKey| *key {
                        {slot(keys, rows, key)}
                    }
                }
            }
        }
    }
}

/// The box that places the row for `key` at its place in the tree.
fn slot(keys: Memo<Vec<RowKey>>, rows: Memo<Vec<Row>>, key: RowKey) -> AnyView {
    let place = move || keys.with(|keys| keys.iter().position(|k| *k == key).unwrap_or(0));
    view! {
        box(class = "ms-slot", style:transform = move || offset(place())) {
            {line(rows, key)}
        }
    }
    .any()
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
