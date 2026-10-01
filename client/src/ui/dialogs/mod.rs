//! The dialogs: connect, new channel and poke.

mod all;
mod connect;
mod create_channel;
mod host;
mod poke;

pub use crate::ui::dialogs::all::DialogsProps;
#[cfg(feature = "screen-share")]
pub use crate::ui::dialogs::host::ModalHostProps;
