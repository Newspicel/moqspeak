//! One session with a moqspeak Worker: a control WebSocket carrying JSON, and a MoQ media session
//! beside it carrying voice and screens.

use std::collections::hash_map::Entry;
use std::collections::{BTreeSet, HashMap};
use std::sync::atomic::Ordering;
use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc::unbounded_channel;
use tokio_tungstenite::tungstenite::Message;

use crate::audio::Audio;
use crate::engine::{Command, platform};
use crate::model::{Client, ClientId};
use crate::screen::Share;
#[cfg(feature = "screen-share")]
use crate::screen::ShareEvent;
use crate::transport::moq::media::MediaSession;
use crate::transport::moq::wire::{Handshake, ServerMsg};
#[cfg(feature = "screen-share")]
use crate::transport::{Action, Out};
use crate::transport::{Capabilities, ConnStatus, Event, MediaStatus, Session, Transport, address};

/// A moqspeak Worker, with voice and screens on the MoQ relay it hands out.
pub struct Worker;

impl Transport for Worker {
    type Address = address::Moq;

    const CAPABILITIES: Capabilities = Capabilities::MOQ;

    async fn run(address: &address::Moq, session: Session<'_>) -> Result<Option<Command>> {
        let Session {
            audio,
            identity,
            nickname,
            commands,
            packets,
            out,
        } = session;

        out.log(format!("Connecting to {}…", address.server));
        let (ws, _) = tokio::time::timeout(
            Duration::from_secs(10),
            tokio_tungstenite::connect_async(address.ws_url.as_str()),
        )
        .await
        .context("connection timed out")?
        .context("could not reach the server")?;
        let (mut ws_tx, mut ws_rx) = ws.split();

        let hello = Handshake::Hello {
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
                            let auth = Handshake::Auth { sig: identity.answer(&server, &nonce) };
                            ws_tx.send(Message::text(serde_json::to_string(&auth)?)).await?;
                        }
                        ServerMsg::Welcome { you, relay, token } => {
                            out.emit(Event::Status(ConnStatus::Connected));
                            out.emit(Event::Welcome { you: you.clone() });
                            out.emit(Event::Media(MediaStatus::Connecting));
                            media = Some(MediaSession::start(
                                relay, token, you.media.clone(), audio.mixer.clone(), media_tx.clone(),
                            ));
                            me = Some(you);
                        }
                        ServerMsg::State { server, channels, clients } => {
                            if let Some(m) = &mut me
                                && let Some(updated) = clients.iter().find(|c| c.id == m.id)
                            {
                                *m = updated.clone();
                            }
                            if let (Some(m), Some(session)) = (&me, &media) {
                                sync_subscriptions(m, &clients, &mut subscribed, session, audio);
                            }
                            last_clients = clients.clone();
                            out.emit(Event::State { server, channels, clients });
                        }
                        ServerMsg::Event { text, .. } => out.log(text),
                        ServerMsg::Chat { target, from, from_name, to, text, .. } => {
                            out.emit(Event::Chat { target, from, from_name, to, text });
                        }
                        ServerMsg::Poke { from, from_name, text } => {
                            out.emit(Event::Poke { from, from_name, text });
                        }
                        ServerMsg::Error { message } => out.warn(message),
                    }
                }
                command = commands.recv() => match command {
                    None | Some(Command::Disconnect) => break Ok(None),
                    Some(c @ Command::Connect { .. }) => break Ok(Some(c)),
                    Some(Command::Send(action)) => {
                        if let Err(e) = ws_tx.send(Message::text(serde_json::to_string(&action)?)).await {
                            break Err(anyhow!("connection lost: {e}"));
                        }
                    }
                    #[cfg(feature = "screen-share")]
                    Some(Command::StartShare(source)) => match share.start(source) {
                        Ok(()) => {
                            let _ = ws_tx.send(Message::text(shared(true, &out))).await;
                        }
                        Err(e) => out.warn(format!("Screen sharing failed: {e:#}")),
                    },
                    #[cfg(feature = "screen-share")]
                    Some(Command::StopShare) => {
                        if share.stop() {
                            let _ = ws_tx.send(Message::text(shared(false, &out))).await;
                        }
                    }
                    #[cfg(feature = "screen-share")]
                    Some(Command::Watch { client, view, sink }) => {
                        let path = last_clients.iter().find(|c| c.id == client).map(|c| c.media.clone());
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
                        let _ = ws_tx.send(Message::text(shared(false, &out))).await;
                    }
                },
                Some(packet) = packets.recv() => {
                    if let Some(session) = &media {
                        session.publish(packet);
                    }
                }
                Some(status) = media_rx.recv() => {
                    if let MediaStatus::Failed(e) = &status {
                        out.warn(format!("Voice transport: {e}"));
                    }
                    if status == MediaStatus::Connecting {
                        subscribed.clear();
                        if let (Some(m), Some(session)) = (&me, &media) {
                            sync_subscriptions(m, &last_clients, &mut subscribed, session, audio);
                        }
                    }
                    out.emit(Event::Media(status));
                }
                _ = tick.tick() => {
                    let mut now: BTreeSet<ClientId> =
                        audio.mixer.talking(Duration::from_millis(250)).into_iter().collect();
                    now.remove(&crate::audio::LOOPBACK_PEER);
                    if let Some(m) = &me
                        && audio.shared.transmitting.load(Ordering::Relaxed)
                    {
                        now.insert(m.id);
                    }
                    if now != talking {
                        talking = now.clone();
                        out.emit(Event::Talking(now));
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
            out.emit(Event::Sharing(false));
        }
        if let Some(session) = media {
            session.stop();
        }
        let _ = ws_tx.send(Message::Close(None)).await;
        if result.is_ok() {
            out.log("Disconnected from server");
        }
        // A lost connection is an error, so the caller can reconnect.
        result
    }
}

/// Tells the interface that sharing turned `on`, and returns the message that tells the server.
#[cfg(feature = "screen-share")]
fn shared(on: bool, out: &Out) -> String {
    out.emit(Event::Sharing(on));
    out.log(if on {
        "You are now sharing your screen"
    } else {
        "Screen sharing stopped"
    });
    serde_json::to_string(&Action::Sharing { sharing: on }).unwrap_or_default()
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
        .map(|c| (c.id, c.media.clone()))
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
        if let Entry::Vacant(slot) = subscribed.entry(id) {
            session.subscribe(id, path.clone());
            slot.insert(path);
        }
    }
    session.set_subscription_count(subscribed.len());
}
