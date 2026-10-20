//! The contract a transport meets. Each one owns its session loop and reaches the rest of the
//! client only through the commands it reads, the events it sends and the audio it is handed.

use std::cell::Cell;
use std::sync::Arc;

use anyhow::Result;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::audio::{Audio, Packet};
use crate::engine::Command;
use crate::identity::Identity;
use crate::transport::{Capabilities, Event};

/// The way to the interface. Cloning one is cheap, which is what a transport that reports from a
/// callback needs.
#[derive(Clone)]
pub struct Out(UnboundedSender<Event>);

impl Out {
    pub fn new(events: UnboundedSender<Event>) -> Self {
        Self(events)
    }

    /// Tells the interface what happened.
    pub fn emit(&self, event: Event) {
        let _ = self.0.send(event);
    }

    /// Writes a line to the log.
    pub fn log(&self, text: impl Into<String>) {
        self.emit(Event::Log {
            error: false,
            text: text.into(),
        });
    }

    /// Writes a line to the log and announces it.
    pub fn warn(&self, text: impl Into<String>) {
        self.emit(Event::Log {
            error: true,
            text: text.into(),
        });
    }
}

/// Everything one session borrows while it runs. A transport takes it apart first, so that the
/// commands and the microphone can be read in the same `select!`.
pub struct Session<'a> {
    pub audio: &'a Arc<Audio>,
    pub identity: &'a Identity,
    pub nickname: &'a str,
    pub commands: &'a mut UnboundedReceiver<Command>,
    pub packets: &'a mut UnboundedReceiver<Packet>,
    pub out: Out,
    /// Set by the transport once the server has taken it. Only a session that reached this is
    /// worth reconnecting, so a bad address fails at once instead of retrying for a minute.
    pub reached: &'a Cell<bool>,
}

/// One way of reaching a server.
pub trait Transport {
    /// Where this transport connects.
    type Address;

    /// What the interface may offer while this transport is live.
    const CAPABILITIES: Capabilities;

    /// Runs one session until it ends. Returns a command that ended the session and should run
    /// next, which is how connecting to another server replaces this one.
    async fn run(address: &Self::Address, session: Session<'_>) -> Result<Option<Command>>;
}
