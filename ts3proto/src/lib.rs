//! The TeamSpeak 3 client protocol. A [`Client`] resolves an address, performs the Init1 and
//! crypto handshake, then exchanges commands, notifications and Opus voice over one UDP socket.
//! Callers register [`Event`] handlers and send commands through the helpers in this module.
//!
//! Vendored from `tsclient-rs` 0.1.6 (MIT), with file transfer and the HTTP nickname lookup
//! removed. See `LICENSE-tsclient-rs`.

// Clippy style nags that don't affect correctness, kept to hold the vendored source close to its
// origin and keep future updates a readable diff.
#![allow(
    clippy::collapsible_if,
    clippy::too_many_arguments,
    clippy::await_holding_lock,
    clippy::single_match,
    clippy::unnecessary_unwrap,
    clippy::manual_range_contains,
    clippy::needless_range_loop,
    clippy::manual_div_ceil,
    clippy::manual_is_multiple_of,
    clippy::upper_case_acronyms,
    clippy::inherent_to_string,
    clippy::never_loop,
    clippy::needless_borrow,
    clippy::clone_on_copy,
    clippy::needless_borrows_for_generic_args,
    clippy::redundant_guards,
    clippy::collapsible_else_if,
    clippy::identity_op,
    clippy::explicit_auto_deref,
    clippy::question_mark,
    clippy::new_without_default
)]

mod api;
mod client;
mod command;
mod commands;
mod crypto;
mod discovery;
mod errors;
mod events;
mod handshake;
mod helpers;
mod notifications;
mod throttle;
mod transport;
mod types;

pub use client::{Client, ClientStatus};
pub use types::ClientState;

pub use types::{
    AbortSignal, AddrResolver, ChannelInfo, ClientInfo, ClientLeftViewEvent, ClientMovedEvent,
    ClientOptions, CommandMiddleware, ConsoleLogger, EscapedString, Event, EventHandler,
    EventMiddleware, KickReason, Logger, NoopLogger, PokeEvent, ResolvedAddr, TextMessage,
    VersionSign, VoiceData,
};

pub use errors::Error;

pub use errors::{
    AlreadyConnectedError, CommandTimeoutError, CryptoInitError, EAXTagMismatchError,
    FakeSignatureMismatchError, InvalidIdentityError, ServerError, TeamspeakError,
};

pub use api::{
    ban_client, client_kick, client_move, client_update, get_client_info, list_channels,
    list_clients, poke, send_text_message,
};

pub use command::{escape, unescape};

pub use crypto::{Identity, generate_identity, get_uid_from_public_key, identity_from_string};

/// The Opus codecs TeamSpeak voice packets carry.
pub mod codec {
    /// Opus tuned for speech: one mono channel.
    pub const OPUS_VOICE: i32 = 4;
    /// Opus tuned for music: two channels.
    pub const OPUS_MUSIC: i32 = 5;
}

/// The `targetmode` values [`send_text_message`](crate::send_text_message) accepts.
pub mod target {
    /// One client, addressed by its id.
    pub const CLIENT: i32 = 1;
    /// Everybody in a channel.
    pub const CHANNEL: i32 = 2;
    /// Everybody on the server.
    pub const SERVER: i32 = 3;
}
