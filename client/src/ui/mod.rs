//! The interface: a channel tree with your voice controls floating over it, a chat beside it
//! that opens when there is a conversation, and a log under it on request.

#![allow(
    clippy::redundant_closure,
    reason = "`view!` reads its conditions and lists through closure syntax"
)]

mod assets;
mod chat;
mod dialogs;
mod frame;
mod head;
mod home;
mod log;
mod notices;
mod parts;
mod pill;
mod root;
mod screen;
mod settings;
pub mod state;
mod theme;
mod tree;

pub use crate::ui::assets::sheet;
pub use crate::ui::root::RootProps;
pub use crate::ui::state::AppState;
