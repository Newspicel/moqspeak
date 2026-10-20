//! The network side of the client. One thread runs a tokio runtime, one transport runs on it, and
//! the interface reaches it through [`Command`]s in and [`Event`]s out. Nothing here knows which
//! protocol carries a session; [`Address`] decides that and the transport does the rest.

use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

use crate::audio::{Audio, Packet};
use crate::identity::Identity;
#[cfg(feature = "screen-share")]
use crate::model::ClientId;
#[cfg(feature = "screen-share")]
use crate::screen::{Source, VideoFrame};
use crate::transport::moq::Worker;
use crate::transport::ts3::TeamSpeak;
use crate::transport::{
    Action, Address, Capabilities, ConnStatus, Event, MediaStatus, Out, Session, Transport,
};

/// What the interface asks the network to do.
#[derive(Clone, Debug)]
pub enum Command {
    Connect {
        address: String,
        nickname: String,
    },
    Disconnect,
    Send(Action),
    /// Shares `source` and replaces any running share.
    #[cfg(feature = "screen-share")]
    StartShare(Source),
    #[cfg(feature = "screen-share")]
    StopShare,
    /// Streams `client`'s screen into `sink` until [`Command::Unwatch`] names the same `view`.
    /// Each viewer has its own `view`, so two views of one screen do not end each other.
    #[cfg(feature = "screen-share")]
    Watch {
        client: ClientId,
        view: u64,
        sink: std::sync::mpsc::Sender<VideoFrame>,
    },
    #[cfg(feature = "screen-share")]
    Unwatch {
        view: u64,
    },
}

/// The handle the interface keeps.
#[derive(Clone)]
pub struct Engine {
    commands: UnboundedSender<Command>,
    pub audio: Arc<Audio>,
}

impl Engine {
    pub fn send(&self, command: Command) {
        let _ = self.commands.send(command);
    }

    /// Starts the runtime and the audio devices. Events arrive on the returned receiver.
    pub fn start(
        input: Option<String>,
        output: Option<String>,
    ) -> (Self, UnboundedReceiver<Event>) {
        let (packet_tx, packet_rx) = unbounded_channel();
        let audio = Arc::new(Audio::start(packet_tx, input, output));
        Self::start_with(audio, packet_rx, Identity::load_or_create())
    }

    /// Starts the runtime around audio the caller already set up.
    pub fn start_with(
        audio: Arc<Audio>,
        packet_rx: UnboundedReceiver<Packet>,
        identity: Identity,
    ) -> (Self, UnboundedReceiver<Event>) {
        let (cmd_tx, cmd_rx) = unbounded_channel();
        let (event_tx, event_rx) = unbounded_channel();
        if let Some(error) = audio.device_info().error {
            let _ = event_tx.send(Event::Log {
                error: true,
                text: format!("Audio: {error}"),
            });
        }
        let worker_audio = audio.clone();
        std::thread::Builder::new()
            .name("network".into())
            .spawn(move || {
                let runtime = tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(2)
                    .enable_all()
                    .build()
                    .expect("tokio runtime");
                runtime.block_on(run(
                    worker_audio,
                    cmd_rx,
                    Out::new(event_tx),
                    packet_rx,
                    identity,
                ));
            })
            .expect("spawn network thread");
        (
            Self {
                commands: cmd_tx,
                audio,
            },
            event_rx,
        )
    }
}

async fn run(
    audio: Arc<Audio>,
    mut commands: UnboundedReceiver<Command>,
    out: Out,
    mut packets: UnboundedReceiver<Packet>,
    identity: Identity,
) {
    const MAX_RETRIES: u32 = 6;
    let mut pending: Option<Command> = None;
    let mut attempts = 0u32;
    loop {
        let next = match pending.take() {
            Some(c) => Some(c),
            None => tokio::select! {
                c = commands.recv() => c,
                // Drop microphone packets while offline.
                Some(_) = packets.recv() => continue,
            },
        };
        let Some(command) = next else { return };
        match command {
            Command::Connect { address, nickname } => {
                let started = std::time::Instant::now();
                let outcome = connect(
                    &audio,
                    &identity,
                    &address,
                    &nickname,
                    &mut commands,
                    &out,
                    &mut packets,
                )
                .await;
                // A session that held for a while earns a fresh set of retries.
                if started.elapsed() > Duration::from_secs(30) {
                    attempts = 0;
                }
                match outcome {
                    Ok(next) => {
                        attempts = 0;
                        out.emit(Event::Status(ConnStatus::Disconnected));
                        pending = next;
                    }
                    Err(e) if attempts < MAX_RETRIES => {
                        attempts += 1;
                        let wait = Duration::from_secs(1 << attempts.min(4));
                        out.warn(format!("{e:#}. Reconnecting in {} s…", wait.as_secs()));
                        out.emit(Event::Media(MediaStatus::Off));
                        out.emit(Event::Talking(BTreeSet::new()));
                        out.emit(Event::Status(ConnStatus::Connecting(address.clone())));
                        // Wait, unless the user decides something else first.
                        tokio::select! {
                            c = commands.recv() => match c {
                                Some(Command::Disconnect) | None => {
                                    attempts = 0;
                                    out.emit(Event::Status(ConnStatus::Disconnected));
                                }
                                other => pending = other,
                            },
                            _ = tokio::time::sleep(wait) => {
                                pending = Some(Command::Connect { address, nickname });
                            }
                        }
                        continue;
                    }
                    Err(e) => {
                        attempts = 0;
                        let text = format!("{e:#}");
                        out.warn(text.clone());
                        out.emit(Event::Status(ConnStatus::Failed(text)));
                    }
                }
                out.emit(Event::Caps(Capabilities::NONE));
                out.emit(Event::Media(MediaStatus::Off));
                out.emit(Event::Talking(BTreeSet::new()));
            }
            #[cfg(feature = "screen-share")]
            Command::StartShare(_) => {
                out.warn("Connect to a server before sharing your screen");
            }
            _ => {}
        }
    }
}

/// Runs one session on whichever transport the address names.
async fn connect(
    audio: &Arc<Audio>,
    identity: &Identity,
    address: &str,
    nickname: &str,
    commands: &mut UnboundedReceiver<Command>,
    out: &Out,
    packets: &mut UnboundedReceiver<Packet>,
) -> Result<Option<Command>> {
    let address = Address::parse(address)?;
    out.emit(Event::Status(ConnStatus::Connecting(
        address.label().to_owned(),
    )));
    let session = Session {
        audio,
        identity,
        nickname,
        commands,
        packets,
        out: out.clone(),
    };
    match address {
        Address::Moq(a) => {
            out.emit(Event::Caps(Worker::CAPABILITIES));
            Worker::run(&a, session).await
        }
        Address::Ts3(a) => {
            out.emit(Event::Caps(TeamSpeak::CAPABILITIES));
            TeamSpeak::run(&a, session).await
        }
    }
}

/// What this client calls the system it runs on.
pub(crate) fn platform() -> String {
    match std::env::consts::OS {
        "macos" => "macOS".into(),
        "windows" => "Windows".into(),
        "linux" => "Linux".into(),
        other => other.into(),
    }
}
