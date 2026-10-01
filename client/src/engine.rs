//! The network side: the control WebSocket, the MoQ media session, and the glue between them
//! and the audio engine. It runs on a tokio runtime of its own and talks to the UI through
//! [`Command`]s in and [`Event`]s out.

use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use tokio_tungstenite::tungstenite::Message;

use crate::audio::{Audio, Packet};
use crate::identity::Identity;
use crate::media::MediaSession;
use crate::model::{Channel, ChatTarget, Client, ClientId, ClientMsg, ServerInfo, ServerMsg};
use crate::screen::Share;
#[cfg(feature = "screen-share")]
use crate::screen::{ShareEvent, Source, VideoFrame};

/// Where the client is in connecting to a server.
#[derive(Clone, Debug, PartialEq)]
pub enum ConnStatus {
    Disconnected,
    Connecting(String),
    Connected,
    Failed(String),
}

/// What the network tells the UI.
#[derive(Clone, Debug)]
#[allow(dead_code, reason = "the bot prints events through Debug")]
pub enum Event {
    Status(ConnStatus),
    Welcome {
        you: Client,
    },
    State {
        server: ServerInfo,
        channels: Vec<Channel>,
        clients: Vec<Client>,
    },
    Log {
        error: bool,
        text: String,
    },
    Chat {
        target: ChatTarget,
        from: ClientId,
        from_name: String,
        to: Option<ClientId>,
        text: String,
    },
    Poke {
        from: ClientId,
        from_name: String,
        text: String,
    },
    Talking(BTreeSet<ClientId>),
    Media(MediaStatus),
    /// Whether this client is sharing its screen right now.
    #[cfg(feature = "screen-share")]
    Sharing(bool),
}

#[derive(Clone, Debug, PartialEq)]
pub enum MediaStatus {
    Off,
    Connecting,
    Connected { relay: String, subscriptions: usize },
    Failed(String),
}

/// What the UI asks the network to do.
#[derive(Clone, Debug)]
pub enum Command {
    Connect {
        address: String,
        nickname: String,
    },
    Disconnect,
    Send(ClientMsg),
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

/// A connection target parsed from what the user typed, such as
/// `moqspeak.example.workers.dev/myserver`.
#[derive(Clone, Debug, PartialEq)]
pub struct Address {
    pub ws_url: String,
    pub server: String,
}

impl Address {
    pub fn parse(input: &str) -> Result<Self> {
        let input = input.trim();
        if input.is_empty() {
            return Err(anyhow!("enter a server address"));
        }
        let (scheme, rest) = if let Some(r) = input.strip_prefix("wss://") {
            ("wss", r)
        } else if let Some(r) = input.strip_prefix("ws://") {
            ("ws", r)
        } else if let Some(r) = input.strip_prefix("https://") {
            ("wss", r)
        } else if let Some(r) = input.strip_prefix("http://") {
            ("ws", r)
        } else if input.starts_with("localhost") || input.starts_with("127.0.0.1") {
            ("ws", input)
        } else {
            ("wss", input)
        };
        let rest = rest.trim_end_matches('/');
        let (host, path) = rest.split_once('/').unwrap_or((rest, ""));
        let server = path
            .trim_start_matches("s/")
            .split('/')
            .next_back()
            .unwrap_or("");
        let server = if server.is_empty() { "public" } else { server };
        if !server
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "_.-".contains(c))
        {
            return Err(anyhow!(
                "server names use letters, digits, '.', '-' and '_'"
            ));
        }
        Ok(Self {
            ws_url: format!("{scheme}://{host}/s/{server}"),
            server: server.to_owned(),
        })
    }
}

/// The handle the UI keeps.
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
                runtime.block_on(run(worker_audio, cmd_rx, event_tx, packet_rx, identity));
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
    events: UnboundedSender<Event>,
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
                let outcome = session(
                    &audio,
                    &identity,
                    &address,
                    &nickname,
                    &mut commands,
                    &events,
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
                        let _ = events.send(Event::Status(ConnStatus::Disconnected));
                        pending = next;
                    }
                    Err(e) if attempts < MAX_RETRIES => {
                        attempts += 1;
                        let wait = Duration::from_secs(1 << attempts.min(4));
                        let _ = events.send(Event::Log {
                            error: true,
                            text: format!("{e:#}. Reconnecting in {} s…", wait.as_secs()),
                        });
                        let _ = events.send(Event::Media(MediaStatus::Off));
                        let _ = events.send(Event::Talking(BTreeSet::new()));
                        let _ = events.send(Event::Status(ConnStatus::Connecting(address.clone())));
                        // Wait, unless the user decides something else first.
                        tokio::select! {
                            c = commands.recv() => match c {
                                Some(Command::Disconnect) | None => {
                                    attempts = 0;
                                    let _ = events.send(Event::Status(ConnStatus::Disconnected));
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
                        let _ = events.send(Event::Log {
                            error: true,
                            text: text.clone(),
                        });
                        let _ = events.send(Event::Status(ConnStatus::Failed(text)));
                    }
                }
                let _ = events.send(Event::Media(MediaStatus::Off));
                let _ = events.send(Event::Talking(BTreeSet::new()));
            }
            #[cfg(feature = "screen-share")]
            Command::StartShare(_) => {
                let _ = events.send(Event::Log {
                    error: true,
                    text: "Connect to a server before sharing your screen".into(),
                });
            }
            _ => {}
        }
    }
}

fn platform() -> String {
    match std::env::consts::OS {
        "macos" => "macOS".into(),
        "windows" => "Windows".into(),
        "linux" => "Linux".into(),
        other => other.into(),
    }
}

/// One connection to one server. Returns a command that ended it and should run next, if any.
async fn session(
    audio: &Arc<Audio>,
    identity: &Identity,
    address: &str,
    nickname: &str,
    commands: &mut UnboundedReceiver<Command>,
    events: &UnboundedSender<Event>,
    packets: &mut UnboundedReceiver<Packet>,
) -> Result<Option<Command>> {
    let address = Address::parse(address)?;
    let _ = events.send(Event::Status(ConnStatus::Connecting(
        address.server.clone(),
    )));
    let _ = events.send(Event::Log {
        error: false,
        text: format!("Connecting to {}…", address.server),
    });
    let (ws, _) = tokio::time::timeout(
        Duration::from_secs(10),
        tokio_tungstenite::connect_async(address.ws_url.as_str()),
    )
    .await
    .context("connection timed out")?
    .context("could not reach the server")?;
    let (mut ws_tx, mut ws_rx) = ws.split();

    let hello = ClientMsg::Hello {
        name: nickname.to_owned(),
        uid: identity.uid(),
        platform: platform(),
        version: env!("CARGO_PKG_VERSION").to_owned(),
        channel: None,
    };
    ws_tx
        .send(Message::text(serde_json::to_string(&hello)?))
        .await?;

    let mut me: Option<Client> = None;
    let mut media: Option<MediaSession> = None;
    let (media_tx, mut media_rx) = unbounded_channel::<MediaStatus>();
    let mut subscribed: HashMap<ClientId, String> = HashMap::new();
    let mut talking = BTreeSet::new();
    let mut tick = tokio::time::interval(Duration::from_millis(60));
    let mut keepalive = tokio::time::interval(Duration::from_secs(20));
    let mut last_clients: Vec<Client> = Vec::new();
    let mut share = Share::default();

    let result = loop {
        tokio::select! {
            frame = ws_rx.next() => {
                let Some(frame) = frame else { break Err(anyhow!("the server closed the connection")) };
                let text = match frame {
                    Ok(Message::Text(t)) => t.to_string(),
                    Ok(Message::Close(_)) => break Err(anyhow!("the server closed the connection")),
                    Ok(_) => continue,
                    Err(e) => break Err(anyhow!("connection lost: {e}")),
                };
                if text == "pong" { continue; }
                let msg: ServerMsg = match serde_json::from_str(&text) {
                    Ok(m) => m,
                    Err(e) => { tracing::warn!("bad server message: {e}: {text}"); continue; }
                };
                match msg {
                    ServerMsg::Challenge { nonce, server } => {
                        let auth = ClientMsg::Auth { sig: identity.answer(&server, &nonce) };
                        ws_tx.send(Message::text(serde_json::to_string(&auth)?)).await?;
                    }
                    ServerMsg::Welcome { you, relay, token } => {
                        let _ = events.send(Event::Status(ConnStatus::Connected));
                        let _ = events.send(Event::Welcome { you: you.clone() });
                        let _ = events.send(Event::Media(MediaStatus::Connecting));
                        media = Some(MediaSession::start(
                            relay, token, you.broadcast.clone(), audio.mixer.clone(), media_tx.clone(),
                        ));
                        me = Some(you);
                    }
                    ServerMsg::State { server, channels, clients } => {
                        if let Some(m) = &mut me {
                            if let Some(updated) = clients.iter().find(|c| c.id == m.id) {
                                *m = updated.clone();
                            }
                        }
                        if let (Some(m), Some(session)) = (&me, &media) {
                            sync_subscriptions(m, &clients, &mut subscribed, session, audio);
                        }
                        last_clients = clients.clone();
                        let _ = events.send(Event::State { server, channels, clients });
                    }
                    ServerMsg::Event { text, .. } => {
                        let _ = events.send(Event::Log { error: false, text });
                    }
                    ServerMsg::Chat { target, from, from_name, to, text, .. } => {
                        let _ = events.send(Event::Chat { target, from, from_name, to, text });
                    }
                    ServerMsg::Poke { from, from_name, text } => {
                        let _ = events.send(Event::Poke { from, from_name, text });
                    }
                    ServerMsg::Error { message } => {
                        let _ = events.send(Event::Log { error: true, text: message });
                    }
                }
            }
            command = commands.recv() => match command {
                None | Some(Command::Disconnect) => break Ok(None),
                Some(c @ Command::Connect { .. }) => break Ok(Some(c)),
                Some(Command::Send(msg)) => {
                    if let Err(e) = ws_tx.send(Message::text(serde_json::to_string(&msg)?)).await {
                        break Err(anyhow!("connection lost: {e}"));
                    }
                }
                #[cfg(feature = "screen-share")]
                Some(Command::StartShare(source)) => match share.start(source) {
                    Ok(()) => {
                        let _ = ws_tx.send(Message::text(shared(true, events))).await;
                    }
                    Err(e) => {
                        let _ = events.send(Event::Log { error: true, text: format!("Screen sharing failed: {e:#}") });
                    }
                },
                #[cfg(feature = "screen-share")]
                Some(Command::StopShare) => {
                    if share.stop() {
                        let _ = ws_tx.send(Message::text(shared(false, events))).await;
                    }
                }
                #[cfg(feature = "screen-share")]
                Some(Command::Watch { client, view, sink }) => {
                    let path = last_clients.iter().find(|c| c.id == client).map(|c| c.broadcast.clone());
                    if let (Some(path), Some(session)) = (path, &media) {
                        session.watch(view, path, sink);
                    }
                }
                #[cfg(feature = "screen-share")]
                Some(Command::Unwatch { view }) => {
                    if let Some(session) = &media {
                        session.unwatch(view);
                    }
                }
            },
            event = share.next() => match event {
                #[cfg(feature = "screen-share")]
                ShareEvent::Frame(frame) => {
                    if let Some(session) = &media {
                        session.publish_video(frame);
                    }
                }
                #[cfg(feature = "screen-share")]
                ShareEvent::Ended => {
                    let _ = ws_tx.send(Message::text(shared(false, events))).await;
                }
            },
            Some(packet) = packets.recv() => {
                if let Some(session) = &media {
                    session.publish(packet);
                }
            }
            Some(status) = media_rx.recv() => {
                if let MediaStatus::Failed(e) = &status {
                    let _ = events.send(Event::Log { error: true, text: format!("Voice transport: {e}") });
                }
                if status == MediaStatus::Connecting {
                    subscribed.clear();
                    if let (Some(m), Some(session)) = (&me, &media) {
                        sync_subscriptions(m, &last_clients, &mut subscribed, session, audio);
                    }
                }
                let _ = events.send(Event::Media(status));
            }
            _ = tick.tick() => {
                let mut now: BTreeSet<ClientId> =
                    audio.mixer.talking(Duration::from_millis(250)).into_iter().collect();
                now.remove(&crate::audio::LOOPBACK_PEER);
                if let Some(m) = &me {
                    if audio.shared.transmitting.load(Ordering::Relaxed) {
                        now.insert(m.id);
                    }
                }
                if now != talking {
                    talking = now.clone();
                    let _ = events.send(Event::Talking(now));
                }
            }
            _ = keepalive.tick() => {
                let _ = ws_tx.send(Message::text("ping")).await;
            }
        }
    };

    for id in subscribed.keys() {
        audio.mixer.remove(*id);
    }
    #[cfg(feature = "screen-share")]
    if share.stop() {
        let _ = events.send(Event::Sharing(false));
    }
    if let Some(session) = media {
        session.stop();
    }
    let _ = ws_tx.send(Message::Close(None)).await;
    if result.is_ok() {
        let _ = events.send(Event::Log {
            error: false,
            text: "Disconnected from server".into(),
        });
    }
    // A lost connection is an error, so the caller can reconnect.
    result
}

/// Tells the UI that sharing turned `on`, and returns the message that tells the server.
#[cfg(feature = "screen-share")]
fn shared(on: bool, events: &UnboundedSender<Event>) -> String {
    let _ = events.send(Event::Sharing(on));
    let text = if on {
        "You are now sharing your screen"
    } else {
        "Screen sharing stopped"
    };
    let _ = events.send(Event::Log {
        error: false,
        text: text.into(),
    });
    serde_json::to_string(&ClientMsg::Sharing { sharing: on }).unwrap_or_default()
}

/// Listens to every other client in my channel, and to nobody else.
fn sync_subscriptions(
    me: &Client,
    clients: &[Client],
    subscribed: &mut HashMap<ClientId, String>,
    session: &MediaSession,
    audio: &Audio,
) {
    let wanted: HashMap<ClientId, String> = clients
        .iter()
        .filter(|c| c.id != me.id && c.channel == me.channel)
        .map(|c| (c.id, c.broadcast.clone()))
        .collect();
    subscribed.retain(|id, path| {
        let keep = wanted.get(id) == Some(path);
        if !keep {
            session.unsubscribe(*id);
            audio.mixer.remove(*id);
        }
        keep
    });
    for (id, path) in wanted {
        if !subscribed.contains_key(&id) {
            session.subscribe(id, path.clone());
            subscribed.insert(id, path);
        }
    }
    session.set_subscription_count(subscribed.len());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_addresses() {
        let a = Address::parse("moqspeak.example.workers.dev/demo").unwrap();
        assert_eq!(a.ws_url, "wss://moqspeak.example.workers.dev/s/demo");
        let a = Address::parse("https://x.dev/s/team/").unwrap();
        assert_eq!(a.ws_url, "wss://x.dev/s/team");
        let a = Address::parse("localhost:8787").unwrap();
        assert_eq!(a.ws_url, "ws://localhost:8787/s/public");
        assert!(Address::parse("x.dev/bad name").is_err());
    }
}
