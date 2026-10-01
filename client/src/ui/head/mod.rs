//! The head of the server column: the server key and its menu, where you are, and the keys for
//! the log and the chat.

mod bar;
mod keys;
mod server_menu;
pub mod tone;
mod where_;

pub use crate::ui::head::bar::HeadProps;
