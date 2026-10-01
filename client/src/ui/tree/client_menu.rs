//! What a client's menu offers. For yourself: your own voice. For another: who they are, how
//! loud they are to you, a message, a poke, a move, and for moderators the rest.

use zgui::prelude::*;
use zgui::reactive::{LocalStorage, UnsyncCallback};
use zgui_ui::prelude::*;

use crate::model::{ClientId, ClientMsg, Role};
use crate::ui::parts::{Erase, act};
use crate::ui::screen::pop_out;
use crate::ui::state::{AppState, ClientRow, Modal};

/// How long ago a millisecond timestamp was, in its largest unit.
fn since(ms: u64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let secs = now.saturating_sub(ms) / 1000;
    match secs {
        0..=59 => "just now".into(),
        60..=3599 => format!("{}m", secs / 60),
        3600..=86399 => format!("{}h {}m", secs / 3600, (secs % 3600) / 60),
        _ => format!("{}d", secs / 86400),
    }
}

/// The name, the role and how long the client has been here, at the head of the menu.
fn facts(state: AppState, id: ClientId) -> AnyView {
    let Some(client) = state.client(id) else {
        return ().any();
    };
    let mut line = format!(
        "{} · online {}",
        client.role.label(),
        since(client.connected_at)
    );
    if !client.version.is_empty() {
        line.push_str(&format!(" · {} {}", client.version, client.platform));
    }
    view! {
        column(class = "ms-menu-facts") {
            text(class = "ms-menu-facts__name") {{client.name}}
            text {{line}}
        }
    }
    .any()
}

/// The volume slider for another client, in percent.
fn volume(state: AppState, id: ClientId) -> AnyView {
    let level = RwSignal::new_local(
        f64::from(
            state
                .volumes
                .with_untracked(|v| v.get(&id).copied().unwrap_or(1.0)),
        ) * 100.0,
    );
    view! {
        row(class = "ms-menu-volume") {
            Slider(
                value = level,
                min = 0.0,
                max = 200.0,
                step = 5.0,
                label = "Volume",
                class = "ms-menu-volume__slider",
                on_change = UnsyncCallback::new(move |v: f64| state.set_volume(id, (v / 100.0) as f32))
            )
            text(class = "ms-menu-volume__value") {{move || format!("{:.0}%", level.get())}}
        }
    }
    .any()
}

/// The menu you get on your own row.
fn own_menu(state: AppState) -> AnyView {
    view! {
        ContextMenuContent {
            MenuItem(on_select = act(move || state.set_mic_muted(!state.mic_muted.get_untracked()))) {
                {move || if state.mic_muted.get() { "Unmute microphone" } else { "Mute microphone" }}
            }
            MenuItem(on_select = act(move || state.set_deafened(!state.deafened.get_untracked()))) {
                {move || if state.deafened.get() { "Turn sound on" } else { "Turn sound off" }}
            }
            MenuItem(on_select = act(move || state.set_away(!state.away.get_untracked()))) {
                {move || if state.away.get() { "Back from away" } else { "Set away" }}
            }
        }
    }
    .any()
}

/// The context menu of one client.
#[component]
pub fn ClientMenu(id: ClientId, row: Signal<ClientRow, LocalStorage>) -> impl IntoView {
    let state = AppState::expect();
    if row.with_untracked(|r| r.me) {
        return own_menu(state);
    }
    let locally_muted = move || state.local_mutes.with(|m| m.contains(&id));
    let role = move || row.with(|r| r.role);
    view! {
        ContextMenuContent {
            {facts(state, id)}
            {volume(state, id)}
            MenuSeparator()
            MenuItem(on_select = act(move || state.open_direct(id))) {"Message"}
            MenuItem(on_select = act(move || {
                let name = row.with_untracked(|r| r.name.clone());
                state.modal.set(Modal::Poke { to: id, name });
            })) {"Poke…"}
            if move || row.with(|r| r.sharing) {
                MenuItem(on_select = act(move || pop_out(state, id, row.with_untracked(|r| r.name.clone())))) {"Watch screen"}
            }
            MenuItem(on_select = act(move || state.set_local_mute(id, !locally_muted()))) {
                {move || if locally_muted() { "Unmute for me" } else { "Mute for me" }}
            }
            if move || state.my_role() >= Role::Mod {
                MenuSeparator()
                MenuItem(on_select = act(move || {
                    if let Some(mine) = state.my_client() {
                        state.move_client(id, mine.channel);
                    }
                })) {"Move to my channel"}
            }
            if move || state.my_role() >= Role::Admin {
                MenuSub {
                    MenuSubTrigger {"Role"}
                    MenuSubContent {
                        MenuItem(disabled = role() == Role::Admin, on_select = act(move || state.msg(ClientMsg::SetRole { id, role: Role::Admin }))) {"Admin"}
                        MenuItem(disabled = role() == Role::Mod, on_select = act(move || state.msg(ClientMsg::SetRole { id, role: Role::Mod }))) {"Moderator"}
                        MenuItem(disabled = role() == Role::User, on_select = act(move || state.msg(ClientMsg::SetRole { id, role: Role::User }))) {"User"}
                    }
                }
            }
            if move || state.my_role() >= Role::Mod {
                MenuItem(
                    destructive = true,
                    on_select = act(move || state.msg(ClientMsg::Kick { id, reason: String::new() }))
                ) {"Kick from channel"}
            }
        }
    }
    .any()
}
