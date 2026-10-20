//! The transports moqspeak speaks. A transport turns the interface's [`Action`]s into one
//! server's protocol and that server's traffic back into [`Event`]s, so the interface and the
//! audio engine stay the same whichever protocol carries them.

mod action;
pub mod address;
mod backend;
mod caps;
mod event;
pub mod moq;
pub mod ts3;

pub use action::Action;
pub use address::Address;
pub use backend::{Out, Session, Transport};
pub use caps::Capabilities;
pub use event::{ConnStatus, Event, MediaStatus};
