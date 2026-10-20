//! One session with a TeamSpeak 3 server. A single UDP socket carries the handshake, the commands
//! and the voice, so there is no media session beside this one.

use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use tokio::sync::mpsc::unbounded_channel;
use ts3proto::{ClientOptions, ConsoleLogger, Event as Ts3Event, codec};

use crate::audio::Packet;
use crate::engine::Command;
use crate::model::{ChatTarget, Client, ClientId, Role};
use crate::transport::ts3::commands::{self, Me};
use crate::transport::ts3::identity;
use crate::transport::ts3::roster::{Change, Roster};
use crate::transport::ts3::version;
use crate::transport::ts3::voice::Spurts;
use crate::transport::{
    Capabilities, ConnStatus, Event, MediaStatus, Out, Session, Transport, address,
};

/// How long to wait for the handshake, which includes the server's proof of work puzzle.
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(20);

/// A TeamSpeak 3 server.
pub struct TeamSpeak;

impl Transport for TeamSpeak {
    type Address = address::Ts3;

    const CAPABILITIES: Capabilities = Capabilities::TS3;

    async fn run(address: &address::Ts3, session: Session<'_>) -> Result<Option<Command>> {
        let Session {
            audio,
            identity: _,
            nickname,
            commands: ui_commands,
            packets,
            out,
            reached,
        } = session;

        out.log(format!("Connecting to {} over TeamSpeak…", address.host));

        // Reaching the security level is proof of work, so keep it off the runtime.
        let ts_identity = tokio::task::spawn_blocking(identity::load_or_create)
            .await
            .context("preparing the TeamSpeak identity")?;

        let options = ClientOptions {
            logger: Arc::new(ConsoleLogger),
            server_password: address.password.clone(),
            default_channel: address.channel.clone(),
            default_channel_password: address.channel_password.clone(),
            version: version::for_this_system(),
            ..ClientOptions::default()
        };
        let mut client = ts3proto::Client::new(
            ts_identity,
            address.addr.clone(),
            nickname.to_owned(),
            options,
        );

        let (tx, mut ts_events) = unbounded_channel::<Ts3Event>();
        let forward: ts3proto::EventHandler = Arc::new(move |event| {
            let _ = tx.send(event);
        });
        client
            .on_notification(forward.clone())
            .on_text_message(forward.clone())
            .on_poked(forward.clone())
            .on_voice_data(forward.clone())
            .on_kicked(forward.clone())
            .on_disconnected(forward.clone());

        tokio::time::timeout(HANDSHAKE_TIMEOUT, client.connect())
            .await
            .context("connecting timed out")?
            .context("could not reach the server")?;
        tokio::time::timeout(HANDSHAKE_TIMEOUT, client.wait_connected(None))
            .await
            .context("the TeamSpeak handshake timed out")?
            .context("the server refused the handshake")?;
        reached.set(true);
        let client = client;

        // A client is shown its own channel and no other until it asks. Asking for them all is
        // what makes the people in the rest of the tree visible.
        if let Err(e) = client.exec_command("channelsubscribeall", 5_000).await {
            out.warn(format!("TeamSpeak: only your own channel is shown ({e})"));
        }

        // Everything the server has queued by now describes what was already here: the channels,
        // and everybody standing in them. Counting it is what tells a greeting from an arrival.
        let mut opening = ts_events.len();

        // The server sends the channels and everyone in view by itself, so the roster fills from
        // the stream rather than from any request.
        let mut roster = Roster::new(address.addr.clone());

        let my_id = client.client_id() as ClientId;
        let mut me = Me {
            id: my_id,
            channel: client.channel_id(),
            name: nickname.to_owned(),
        };
        let you = roster
            .client(my_id)
            .cloned()
            .unwrap_or_else(|| placeholder(&me));
        me.channel = you.channel;
        me.name = you.name.clone();

        out.emit(Event::Status(ConnStatus::Connected));
        out.emit(Event::Welcome { you });

        let mut peers = None;
        publish(&roster, &me, &out, address, &mut peers);

        let mut spurts = Spurts::default();
        let mut talking = BTreeSet::new();
        let mut heard: BTreeSet<ClientId> = BTreeSet::new();
        let mut transmitting = false;
        let mut tick = tokio::time::interval(Duration::from_millis(60));

        let result = loop {
            tokio::select! {
                event = ts_events.recv() => {
                    let Some(event) = event else {
                        break Err(anyhow!("the connection ended"));
                    };
                    // Anything still left from the opening is somebody who was already here.
                    let arrived = opening == 0;
                    opening = opening.saturating_sub(1);
                    match event {
                        Ts3Event::Notification { name, params } => {
                            // Read the log line before applying, while a leaving client still
                            // has a name.
                            match name.as_str() {
                                "notifycliententerview" if arrived => {
                                    if let Some(id) = number(&params, "clid")
                                        && id != my_id
                                    {
                                        out.log(format!("{} connected", nickname_of(&params)));
                                    }
                                }
                                "notifyclientleftview" => {
                                    if let Some(id) = number(&params, "clid") {
                                        if let Some(who) = roster.client(id).map(|c| c.name.clone()) {
                                            out.log(format!("{who} disconnected"));
                                        }
                                        audio.mixer.remove(id);
                                        spurts.forget(id);
                                        heard.remove(&id);
                                    }
                                }
                                _ => {}
                            }
                            match roster.apply(&name, &params) {
                                Change::Quiet => {}
                                Change::State => {
                                    if let Some(mine) = roster.client(my_id) {
                                        me.channel = mine.channel;
                                        me.name = mine.name.clone();
                                    }
                                    publish(&roster, &me, &out, address, &mut peers);
                                }
                            }
                        }
                        Ts3Event::TextMessage(message) => {
                            let target = match message.target_mode {
                                ts3proto::target::CLIENT => ChatTarget::Client,
                                ts3proto::target::CHANNEL => ChatTarget::Channel,
                                _ => ChatTarget::Server,
                            };
                            let from = message.invoker_id as ClientId;
                            // The server echoes a private message only to its recipient, so one
                            // arriving here is never our own.
                            if target == ChatTarget::Client && from == my_id {
                                continue;
                            }
                            out.emit(Event::Chat {
                                target,
                                from,
                                from_name: message.invoker_name.clone(),
                                to: (target == ChatTarget::Client).then_some(my_id),
                                text: message.message.clone(),
                            });
                        }
                        Ts3Event::Poked(poke) => {
                            out.emit(Event::Poke {
                                from: poke.invoker_id as ClientId,
                                from_name: poke.invoker_name.clone(),
                                text: poke.message.clone(),
                            });
                        }
                        Ts3Event::VoiceData(voice) => {
                            if voice.codec != codec::OPUS_VOICE && voice.codec != codec::OPUS_MUSIC {
                                tracing::debug!("ignoring voice in codec {}", voice.codec);
                                continue;
                            }
                            let speaker = voice.client_id as ClientId;
                            // An empty frame ends a talk spurt.
                            if voice.data.is_empty() {
                                spurts.forget(speaker);
                                continue;
                            }
                            let seq = spurts.widen(speaker, voice.id);
                            audio.mixer.push(speaker, &Packet { seq, data: voice.data.to_vec() });
                            heard.insert(speaker);
                        }
                        // The notifications above carry these, with the names still intact.
                        Ts3Event::ClientEnter(_)
                        | Ts3Event::ClientLeave(_)
                        | Ts3Event::ClientMoved(_)
                        | Ts3Event::Connected => {}
                        Ts3Event::Kicked(reason) => {
                            break Err(anyhow!("the server removed you: {reason}"));
                        }
                        Ts3Event::Disconnected(error) => {
                            break match error {
                                Some(e) => Err(anyhow!("connection lost: {e}")),
                                None => Ok(None),
                            };
                        }
                    }
                }
                command = ui_commands.recv() => match command {
                    None | Some(Command::Disconnect) => break Ok(None),
                    Some(c @ Command::Connect { .. }) => break Ok(Some(c)),
                    Some(Command::Send(action)) => commands::send(&client, &me, &action, &out).await,
                    #[cfg(feature = "screen-share")]
                    Some(Command::StartShare(_)) => {
                        out.warn("TeamSpeak carries no screens");
                    }
                    #[cfg(feature = "screen-share")]
                    Some(Command::StopShare) => {}
                    #[cfg(feature = "screen-share")]
                    Some(Command::Watch { .. } | Command::Unwatch { .. }) => {}
                },
                Some(packet) = packets.recv() => {
                    client.send_voice(packet.data, codec::OPUS_VOICE);
                }
                _ = tick.tick() => {
                    // Tell the server the moment a talk spurt ends, so the others stop showing it.
                    let now_transmitting = audio.shared.transmitting.load(Ordering::Relaxed);
                    if transmitting && !now_transmitting {
                        client.send_voice(Vec::new(), codec::OPUS_VOICE);
                    }
                    transmitting = now_transmitting;

                    let mut now: BTreeSet<ClientId> =
                        audio.mixer.talking(Duration::from_millis(250)).into_iter().collect();
                    now.remove(&crate::audio::LOOPBACK_PEER);
                    if now_transmitting {
                        now.insert(my_id);
                    }
                    if now != talking {
                        talking = now.clone();
                        out.emit(Event::Talking(now));
                    }
                }
            }
        };

        for speaker in &heard {
            audio.mixer.remove(*speaker);
        }
        let _ = client.disconnect().await;
        if result.is_ok() {
            out.log("Disconnected from server");
        }
        result
    }
}

/// Sends the current lists to the interface, and the voice status when it changed.
fn publish(roster: &Roster, me: &Me, out: &Out, address: &address::Ts3, peers: &mut Option<usize>) {
    out.emit(Event::State {
        server: roster.server.clone(),
        channels: roster.channels.clone(),
        clients: roster.clients.clone(),
    });
    let now = roster
        .clients
        .iter()
        .filter(|c| c.id != me.id && c.channel == me.channel)
        .count();
    if *peers != Some(now) {
        *peers = Some(now);
        out.emit(Event::Media(MediaStatus::Connected {
            relay: address.host.clone(),
            subscriptions: now,
        }));
    }
}

/// A client id from a notification.
fn number(params: &HashMap<String, String>, key: &str) -> Option<ClientId> {
    params.get(key)?.trim().parse().ok()
}

/// The nickname a notification carries, unescaped.
fn nickname_of(params: &HashMap<String, String>) -> String {
    params
        .get("client_nickname")
        .map(|n| ts3proto::unescape(n))
        .unwrap_or_default()
}

/// Who to show as yourself until the client list names you.
fn placeholder(me: &Me) -> Client {
    Client {
        id: me.id,
        uid: String::new(),
        name: me.name.clone(),
        channel: me.channel,
        muted: false,
        deaf: false,
        away: false,
        away_message: String::new(),
        media: String::new(),
        sharing: false,
        role: Role::User,
        rank: String::new(),
        connected_at: 0,
        platform: String::new(),
        version: String::new(),
    }
}
