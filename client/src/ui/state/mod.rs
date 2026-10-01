//! The interface state: everything the network reported, plus what the person chose.

mod actions;
mod app;
mod apply;
mod chat;
mod log;
mod modal;
mod note;
mod pointer;
mod settings;
mod tree;

pub use crate::ui::state::app::AppState;
pub use crate::ui::state::chat::{Conversation, Message, MessageKind};
pub use crate::ui::state::log::LogLine;
pub use crate::ui::state::modal::Modal;
pub use crate::ui::state::note::Level;
pub use crate::ui::state::pointer::{Drag, Selection};
pub use crate::ui::state::settings::{
    Bookmark, CHAT_MAX, CHAT_MIN, DEFAULT_ADDRESS, Settings, mode_from_str, mode_to_str,
};
pub use crate::ui::state::tree::{ChannelRow, ClientRow, Row, RowKey, tree_rows};
