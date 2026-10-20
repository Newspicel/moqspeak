//! The TeamSpeak 3 transport. One UDP socket carries the handshake, the commands and the Opus
//! voice. Channels, clients and messages arrive as the server's own notifications and become the
//! same events every other transport reports.
//!
//! Support is experimental and covers talking, moving, messages and pokes. A TeamSpeak server
//! grants server groups in place of moqspeak's roles, and carries no screens.

mod commands;
mod identity;
mod roster;
mod session;
mod spacer;
mod voice;

pub use session::TeamSpeak;
