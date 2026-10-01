//! Voice over Media over QUIC.
//!
//! Every client publishes one broadcast with one track, `audio`, that carries a group per Opus
//! packet. A client subscribes to the broadcasts of the other clients in its channel. The relay
//! is Cloudflare's draft-16 MoQ relay; the control server hands out its URL and token.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use moq_net::{Pattern, Patterns, Timestamp};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use tokio::task::JoinHandle;

use crate::audio::{Mixer, Packet};
use crate::engine::MediaStatus;
use crate::model::ClientId;

/// The track every broadcast carries its voice on.
pub const AUDIO_TRACK: &str = "audio";

enum Op {
    Publish(Packet),
    Subscribe(ClientId, String),
    Unsubscribe(ClientId),
    Count(usize),
}

/// A handle to the media task. Dropping it or calling [`MediaSession::stop`] ends the task.
pub struct MediaSession {
    ops: UnboundedSender<Op>,
    task: JoinHandle<()>,
}

impl MediaSession {
    /// Starts connecting to `relay` and publishing at `broadcast`. Must run inside a tokio
    /// runtime.
    pub fn start(
        relay: String,
        token: Option<String>,
        broadcast: String,
        mixer: Arc<Mixer>,
        status: UnboundedSender<MediaStatus>,
    ) -> Self {
        let (ops, rx) = unbounded_channel();
        let task = tokio::spawn(async move {
            let host = relay_host(&relay);
            if let Err(e) = run(relay, token, broadcast, mixer, rx, status.clone(), host).await {
                let _ = status.send(MediaStatus::Failed(format!("{e:#}")));
            }
        });
        Self { ops, task }
    }

    pub fn publish(&self, packet: Packet) {
        let _ = self.ops.send(Op::Publish(packet));
    }

    pub fn subscribe(&self, id: ClientId, path: String) {
        let _ = self.ops.send(Op::Subscribe(id, path));
    }

    pub fn unsubscribe(&self, id: ClientId) {
        let _ = self.ops.send(Op::Unsubscribe(id));
    }

    pub fn set_subscription_count(&self, n: usize) {
        let _ = self.ops.send(Op::Count(n));
    }

    pub fn stop(self) {
        self.task.abort();
    }
}

impl Drop for MediaSession {
    fn drop(&mut self) {
        self.task.abort();
    }
}

fn relay_host(relay: &str) -> String {
    url::Url::parse(relay)
        .ok()
        .and_then(|u| u.host_str().map(str::to_owned))
        .unwrap_or_else(|| relay.to_owned())
}

/// The relay URL with the token as its path, which is where Cloudflare reads it.
fn relay_url(relay: &str, token: Option<&str>) -> Result<url::Url> {
    let base = relay.trim_end_matches('/');
    let full = match token {
        Some(t) if !t.is_empty() => format!("{base}/{t}"),
        _ => base.to_owned(),
    };
    url::Url::parse(&full).context("invalid relay URL")
}

fn client() -> Result<moq_tokio::Client> {
    let mut cfg = moq_tokio::connect::Config::default();
    // The relay answers WebTransport only; racing a WebSocket fallback only adds noise.
    cfg.websocket.enabled = Some(false);
    Ok(cfg.init(Default::default())?)
}

/// The namespace the broadcasts of everyone on this server live under.
fn room_of(broadcast: &str) -> &str {
    broadcast
        .rsplit_once('/')
        .map(|(room, _)| room)
        .unwrap_or(broadcast)
}

#[allow(clippy::too_many_arguments)]
async fn run(
    relay: String,
    token: Option<String>,
    broadcast: String,
    mixer: Arc<Mixer>,
    mut ops: UnboundedReceiver<Op>,
    status: UnboundedSender<MediaStatus>,
    host: String,
) -> Result<()> {
    let url = relay_url(&relay, token.as_deref())?;
    let _ = status.send(MediaStatus::Connecting);

    // Publisher: one broadcast, one track, announced before connecting.
    let pub_origin = moq_tokio::origin::spawn();
    let producer = pub_origin.create_broadcast(broadcast.as_str())?;
    let mut track = producer.create_track(AUDIO_TRACK, None)?;
    producer.announce(Default::default())?;

    // Subscriber: scoped to this server's room so discovery asks for that namespace only.
    let sub_origin = moq_tokio::origin::spawn();
    let scope = Patterns::from(Pattern::subtree(room_of(&broadcast))?);
    let sub_scoped = sub_origin.scope("", &scope)?;

    let connect = async {
        let publisher = client()?.with_publisher(&pub_origin).connect(url.clone());
        let subscriber = client()?
            .with_subscriber(sub_scoped.clone())
            .connect(url.clone());
        let publisher = publisher
            .established()
            .await
            .context("publishing to the relay")?;
        let subscriber = subscriber
            .established()
            .await
            .context("subscribing at the relay")?;
        anyhow::Ok((publisher, subscriber))
    };
    let (mut publisher, mut subscriber) = tokio::time::timeout(Duration::from_secs(20), connect)
        .await
        .context("timed out connecting to the MoQ relay")??;

    let consumer = sub_scoped.consume();
    let mut readers: HashMap<ClientId, JoinHandle<()>> = HashMap::new();
    let mut count = 0usize;
    let connected = |count| MediaStatus::Connected {
        relay: host.clone(),
        subscriptions: count,
    };
    let _ = status.send(connected(count));

    loop {
        tokio::select! {
            op = ops.recv() => match op {
                None => break,
                Some(Op::Publish(packet)) => {
                    if let Err(e) = track.write_frame(Timestamp::now(), bytes::Bytes::from(packet.encode())) {
                        tracing::warn!("write audio frame: {e}");
                    }
                }
                Some(Op::Subscribe(id, path)) => {
                    if let Some(old) = readers.remove(&id) {
                        old.abort();
                    }
                    let consumer = consumer.clone();
                    let mixer = mixer.clone();
                    readers.insert(id, tokio::spawn(read_peer(consumer, id, path, mixer)));
                }
                Some(Op::Unsubscribe(id)) => {
                    if let Some(old) = readers.remove(&id) {
                        old.abort();
                    }
                }
                Some(Op::Count(n)) => {
                    count = n;
                    if publisher.connected() && subscriber.connected() {
                        let _ = status.send(connected(count));
                    }
                }
            },
            s = publisher.status() => {
                if report(s, &status, connected(count)) { break; }
            }
            s = subscriber.status() => {
                if report(s, &status, connected(count)) { break; }
            }
        }
    }

    for (_, reader) in readers {
        reader.abort();
    }
    let _ = track.finish();
    producer.close();
    Ok(())
}

fn report(
    s: moq_tokio::Result<moq_tokio::connection::Status>,
    status: &UnboundedSender<MediaStatus>,
    connected: MediaStatus,
) -> bool {
    use moq_tokio::connection::Status;
    let (next, fatal) = match s {
        Ok(Status::Connected) => (connected, false),
        Ok(Status::Disconnected) => (MediaStatus::Connecting, false),
        Ok(_) => return false,
        Err(e) => (MediaStatus::Failed(e.to_string()), true),
    };
    let _ = status.send(next);
    fatal
}

/// Plays one remote broadcast until aborted, resubscribing whenever it ends.
async fn read_peer(
    consumer: moq_net::origin::Consumer,
    id: ClientId,
    path: String,
    mixer: Arc<Mixer>,
) {
    loop {
        if let Err(e) = read_once(&consumer, id, &path, &mixer).await {
            tracing::debug!("audio of {path} ended: {e:#}");
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}

async fn read_once(
    consumer: &moq_net::origin::Consumer,
    id: ClientId,
    path: &str,
    mixer: &Mixer,
) -> Result<()> {
    let broadcast = consumer.routed_broadcast(path).await?;
    let mut sub = broadcast.track(AUDIO_TRACK)?.subscribe(None).await?;
    tracing::info!("listening to {path}");
    while let Some(mut group) = sub.recv_group().await? {
        while let Some(frame) = group.read_frame().await? {
            if let Some(packet) = Packet::decode(&frame.payload) {
                mixer.push(id, &packet);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_relay_url() {
        let u = relay_url("https://relay.example.com", Some("abc.def.ghi")).unwrap();
        assert_eq!(u.as_str(), "https://relay.example.com/abc.def.ghi");
        let u = relay_url("https://relay.example.com/", None).unwrap();
        assert_eq!(u.as_str(), "https://relay.example.com/");
        assert_eq!(room_of("moqspeak/demo/3-ab"), "moqspeak/demo");
    }
}
