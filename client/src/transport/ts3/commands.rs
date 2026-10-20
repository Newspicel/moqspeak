//! Turns the interface's actions into TeamSpeak commands. A command the server refuses is a
//! line in the log, not the end of the session.

use ts3proto::{
    KickReason, client_kick, client_move, client_update, poke, send_text_message, target,
};

use crate::model::{ChannelId, ChatTarget, ClientId};
use crate::transport::{Action, Event, Out};

/// Who I am while a session runs, which several commands need to name.
pub struct Me {
    pub id: ClientId,
    pub channel: ChannelId,
    pub name: String,
}

/// Asks the server for one action.
pub async fn send(client: &ts3proto::Client, me: &Me, action: &Action, out: &Out) {
    let outcome = match action {
        Action::Join { channel } => client_move(client, me.id as i32, *channel, "").await,
        Action::Move { id, channel } => client_move(client, *id as i32, *channel, "").await,
        Action::Poke { to, text } => poke(client, *to as i32, text).await,
        Action::Kick { id, reason } => {
            client_kick(client, *id as i32, KickReason::Server, reason).await
        }
        Action::Rename { name } => client_update(client, &[("client_nickname", name)]).await,
        Action::Status {
            muted,
            deaf,
            away,
            away_message,
        } => {
            let mut params: Vec<(&str, String)> = Vec::new();
            if let Some(muted) = muted {
                params.push(("client_input_muted", flag(*muted)));
            }
            if let Some(deaf) = deaf {
                params.push(("client_output_muted", flag(*deaf)));
            }
            if let Some(away) = away {
                params.push(("client_away", flag(*away)));
            }
            if let Some(message) = away_message {
                params.push(("client_away_message", message.clone()));
            }
            if params.is_empty() {
                return;
            }
            let borrowed: Vec<(&str, &str)> =
                params.iter().map(|(k, v)| (*k, v.as_str())).collect();
            client_update(client, &borrowed).await
        }
        Action::Chat { target, to, text } => {
            let (mode, id) = match target {
                ChatTarget::Server => (target::SERVER, 0),
                ChatTarget::Channel => (target::CHANNEL, me.channel),
                ChatTarget::Client => (target::CLIENT, to.unwrap_or(me.id)),
            };
            let outcome = send_text_message(client, mode, id, text).await;
            // TeamSpeak echoes a channel and a server message back to the sender, so the
            // conversation shows it. A private message it does not, so report that one here.
            if outcome.is_ok() && *target == ChatTarget::Client {
                out.emit(Event::Chat {
                    target: ChatTarget::Client,
                    from: me.id,
                    from_name: me.name.clone(),
                    to: *to,
                    text: text.clone(),
                });
            }
            outcome
        }
        // The interface hides these where the capabilities say a transport has no answer for
        // them. Reaching one anyway is worth a line.
        Action::CreateChannel { .. }
        | Action::EditChannel { .. }
        | Action::DeleteChannel { .. } => {
            out.warn("Editing channels on a TeamSpeak server needs the TeamSpeak client");
            return;
        }
        Action::SetRole { .. } => {
            out.warn("TeamSpeak grants server groups, which moqspeak does not assign");
            return;
        }
        Action::Sharing { .. } => return,
    };
    if let Err(e) = outcome {
        out.warn(describe(action, &e));
    }
}

fn flag(on: bool) -> String {
    if on { "1".into() } else { "0".into() }
}

/// What to tell the person when the server refuses an action.
fn describe(action: &Action, error: &ts3proto::Error) -> String {
    let what = match action {
        Action::Join { .. } => "could not join that channel",
        Action::Move { .. } => "could not move that client",
        Action::Poke { .. } => "could not poke that client",
        Action::Kick { .. } => "could not remove that client",
        Action::Rename { .. } => "could not change your name",
        Action::Status { .. } => "could not change your status",
        Action::Chat { .. } => "could not send that message",
        _ => "the server refused that",
    };
    format!("TeamSpeak: {what} ({error})")
}
