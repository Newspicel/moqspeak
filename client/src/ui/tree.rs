//! The server tree: the server, its channels, and who is in each.

use zgui::prelude::*;

use crate::ui::IntoAny;
use zgui::reactive::UnsyncCallback;
use zgui_ui::prelude::*;

use crate::model::{ChannelId, ClientId, ClientMsg, Role as Perm};
use crate::ui::avatar::UserAvatarProps;
use crate::ui::icons::{self, IcoProps};
use crate::ui::state::{AppState, Drag, Modal, Row, Selection, tree_rows};

fn act(f: impl Fn() + 'static) -> Option<UnsyncCallback<()>> {
    Some(UnsyncCallback::new(move |_: ()| f()))
}

fn indent(depth: u16) -> Option<String> {
    Some(format!("{}px", 6 + depth as u32 * 16))
}

#[component]
pub fn ServerTree() -> impl IntoView {
    let state = AppState::expect();
    let rows = Memo::new(move |_| {
        let server = state.server.get();
        let me = state.me.get();
        let collapsed = state.collapsed.get();
        state.channels.with(|channels| {
            state
                .clients
                .with(|clients| tree_rows(&server, channels, clients, me, &collapsed))
        })
    });

    view! {
        column(class = "pane tree-pane") {
            if move || state.channels.with(Vec::is_empty) {
                Empty(class = "tree-empty") {
                    EmptyHeader {
                        EmptyMedia(variant = EmptyMediaVariant::Icon) {
                            if move || matches!(state.status.get(), crate::engine::ConnStatus::Connecting(_)) {
                                Spinner(label = "Connecting")
                            } else {
                                Ico(svg = icons::SERVER)
                            }
                        }
                        EmptyTitle {{move || match state.status.get() {
                            crate::engine::ConnStatus::Connecting(server) => format!("Connecting to {server}…"),
                            crate::engine::ConnStatus::Failed(_) => "Connection failed".to_owned(),
                            _ => "Not connected".to_owned(),
                        }}}
                        EmptyDescription {{move || match state.status.get() {
                            crate::engine::ConnStatus::Failed(e) => e,
                            _ => "Join a server to see its channels and who is talking.".to_owned(),
                        }}}
                    }
                    EmptyContent {
                        Button(on:click = move |_| state.modal.set(Modal::Connect)) {"Connect…"}
                    }
                }
            } else {
                ScrollArea(class = "tree-scroll", label = "Server tree") {
                    column(class = "tree", a11y:role = Role::Tree, a11y:label = "Server tree") {
                        for row in move || rows.get(), key = |row: &Row| row.clone() {
                            TreeRow(row = row)
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn TreeRow(row: Row) -> impl IntoView {
    match row {
        Row::Server { name } => view! { ServerRow(name = name) }.into_any(),
        Row::Channel { id, name, depth, full, is_default, count, has_children, collapsed, .. } => {
            view! { ChannelRow(id = id, name = name, depth = depth, full = full, is_default = is_default, count = count, has_children = has_children, collapsed = collapsed) }
                .into_any()
        }
        Row::Client { id, name, depth, muted, deaf, away, me, sharing, role } => view! {
            ClientRow(id = id, name = name, depth = depth, muted = muted, deaf = deaf, away = away, me = me, sharing = sharing, role = role)
        }
        .into_any(),
    }
}

#[component]
fn ServerRow(name: String) -> impl IntoView {
    let state = AppState::expect();
    view! {
        ContextMenu {
            ContextMenuTrigger {
                row(
                    class = "node node-server",
                    class:selected = move || state.selected.get() == Selection::Server,
                    a11y:role = Role::TreeItem,
                    a11y:label = name.clone(),
                    style:padding-left = indent(0),
                    on:click = move |_| state.selected.set(Selection::Server)
                ) {
                    Ico(svg = icons::SERVER, class = "srv-ico")
                    text(class = "node-name") {{name.clone()}}
                }
            }
            ContextMenuContent {
                if move || state.my_role() >= Perm::Mod {
                    MenuItem(on_select = act(move || state.modal.set(Modal::CreateChannel { parent: None }))) {"Create Channel…"}
                    MenuSeparator()
                }
                MenuItem(on_select = act(move || state.disconnect())) {"Disconnect"}
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
#[component]
fn ChannelRow(
    id: ChannelId,
    name: String,
    depth: u16,
    full: bool,
    is_default: bool,
    count: usize,
    has_children: bool,
    collapsed: bool,
) -> impl IntoView {
    let state = AppState::expect();
    let mine = move || state.my_client().is_some_and(|c| c.channel == id);
    let dragging = move || state.drag.with(|d| d.as_ref().is_some_and(|d| d.active));
    let label = name.clone();
    view! {
        ContextMenu {
            ContextMenuTrigger {
                row(
                    class = "node node-channel",
                    class:selected = move || state.selected.get() == Selection::Channel(id),
                    class:mine = mine,
                    class:full = full,
                    class:drop = move || dragging() && state.drop_target.get() == Some(id),
                    tabindex = Focus::Sequential,
                    a11y:role = Role::TreeItem,
                    a11y:label = label.clone(),
                    style:padding-left = indent(depth),
                    on:click = move |_| if state.click(Selection::Channel(id)) { state.join(id) },
                    on:key_down = move |ev| match &ev.key {
                        Key::Named(NamedKey::Enter) => state.join(id),
                        Key::Named(NamedKey::ArrowLeft) if !collapsed && has_children => state.toggle_collapsed(id),
                        Key::Named(NamedKey::ArrowRight) if collapsed => state.toggle_collapsed(id),
                        _ => {}
                    },
                    on:pointer_enter = move |_| {
                        if state.drag.with_untracked(|d| d.as_ref().is_some_and(|d| d.active)) {
                            state.drop_target.set(Some(id));
                        }
                    },
                    on:pointer_leave = move |_| if state.drop_target.get_untracked() == Some(id) { state.drop_target.set(None) }
                ) {
                    if move || has_children {
                        control(
                            class = "twisty",
                            class:open = !collapsed,
                            a11y:label = if collapsed { "Expand" } else { "Collapse" },
                            on:click:stop = move |_| state.toggle_collapsed(id),
                            on:double_click:stop = move |_| {}
                        ) { Ico(svg = icons::CHEVRON) }
                    } else {
                        box(class = "twisty-space") {}
                    }
                    Ico(svg = icons::CHANNEL, class = "ch-ico")
                    text(class = "node-name") {{name.clone()}}
                    spacer()
                    if move || is_default {
                        Ico(svg = icons::HOME, class = "flag flag-home")
                    }
                    if move || count > 0 {
                        Badge(variant = BadgeVariant::Secondary, class = "node-badge") {{count.to_string()}}
                    }
                }
            }
            ContextMenuContent {
                MenuItem(on_select = act(move || state.join(id))) {"Switch to Channel"}
                MenuItem(on_select = act(move || state.toggle_collapsed(id))) {{if collapsed { "Expand" } else { "Collapse" }}}
                if move || state.my_role() >= Perm::Mod {
                    MenuSeparator()
                    MenuItem(on_select = act(move || state.modal.set(Modal::CreateChannel { parent: Some(id) }))) {"Create Sub-Channel…"}
                }
                if move || state.my_role() >= Perm::Admin {
                    MenuItem(
                        destructive = true,
                        disabled = is_default,
                        on_select = act(move || state.msg(ClientMsg::DeleteChannel { id }))
                    ) {"Delete Channel"}
                }
            }
        }
    }
}

#[component]
fn ClientRow(
    id: ClientId,
    name: String,
    depth: u16,
    muted: bool,
    deaf: bool,
    away: bool,
    me: bool,
    sharing: bool,
    role: Perm,
) -> impl IntoView {
    let state = AppState::expect();
    let talking = move || state.talking.with(|t| t.contains(&id));
    let locally_muted = move || state.local_mutes.with(|m| m.contains(&id));
    let lit = Signal::derive(move || talking() && !locally_muted());
    // A press that started here and has not become a drag dies with this row.
    on_cleanup_local(move || {
        if state
            .drag
            .with_untracked(|d| d.as_ref().is_some_and(|d| d.client == id && !d.active))
        {
            state.drag.set(None);
        }
    });
    let label = name.clone();
    let drag_name = name.clone();
    let display = name.clone();
    let poke_name = name.clone();

    let menu = if me {
        view! {
            ContextMenuContent {
                MenuItem(on_select = act(move || state.set_away(!state.away.get_untracked()))) {"Toggle Away"}
                MenuItem(on_select = act(move || state.set_mic_muted(!state.mic_muted.get_untracked()))) {"Toggle Microphone"}
                MenuItem(on_select = act(move || state.set_deafened(!state.deafened.get_untracked()))) {"Toggle Speakers"}
            }
        }
        .into_any()
    } else {
        view! {
            ContextMenuContent {
                MenuItem(on_select = act(move || state.open_private(id))) {"Send Text Message"}
                if move || state.my_role() >= Perm::Mod {
                    MenuItem(on_select = act(move || {
                        if let Some(mine) = state.my_client() { state.move_client(id, mine.channel) }
                    })) {"Move to My Channel"}
                }
                MenuItem(on_select = act({
                    let name = poke_name.clone();
                    move || state.modal.set(Modal::Poke { to: id, name: name.clone() })
                })) {"Poke Client…"}
                MenuSeparator()
                MenuItem(on_select = act(move || state.set_local_mute(id, !locally_muted()))) {
                    {move || if locally_muted() { "Unmute Locally" } else { "Mute Locally" }}
                }
                if move || state.my_role() >= Perm::Admin {
                    MenuSeparator()
                    MenuLabel {"Role"}
                    MenuItem(disabled = role == Perm::Admin, on_select = act(move || state.msg(ClientMsg::SetRole { id, role: Perm::Admin }))) {"Make Admin"}
                    MenuItem(disabled = role == Perm::Mod, on_select = act(move || state.msg(ClientMsg::SetRole { id, role: Perm::Mod }))) {"Make Moderator"}
                    MenuItem(disabled = role == Perm::User, on_select = act(move || state.msg(ClientMsg::SetRole { id, role: Perm::User }))) {"Make User"}
                }
                if move || state.my_role() >= Perm::Mod {
                    MenuSeparator()
                    MenuItem(
                        destructive = true,
                        on_select = act(move || state.msg(ClientMsg::Kick { id, reason: String::new() }))
                    ) {"Kick from Channel"}
                }
            }
        }
        .into_any()
    };

    view! {
        ContextMenu {
            ContextMenuTrigger {
                row(
                    class = "node node-client",
                    class:selected = move || state.selected.get() == Selection::Client(id),
                    class:me = me,
                    class:talking = talking,
                    class:away = away,
                    a11y:role = Role::TreeItem,
                    a11y:label = label.clone(),
                    style:padding-left = indent(depth),
                    on:click = move |_| if state.click(Selection::Client(id)) && !me { state.open_private(id) },
                    on:pointer_down = move |ev| {
                        // Anyone can drag themselves; moving others takes a moderator.
                        if ev.button == Some(PointerButton::Primary) && (me || state.my_role_untracked() >= Perm::Mod) {
                            state.drag.set(Some(Drag {
                                client: id,
                                name: drag_name.clone(),
                                origin: (f32::from(ev.position.x), f32::from(ev.position.y)),
                                active: false,
                            }));
                        }
                    }
                ) {
                    UserAvatar(name = display.clone(), talking = lit, dim = away)
                    text(class = "node-name") {{display.clone()}}
                    if move || away {
                        Badge(variant = BadgeVariant::Outline, class = "node-badge") {"away"}
                    }
                    if move || me {
                        Badge(variant = BadgeVariant::Secondary, class = "node-badge") {"you"}
                    }
                    if move || sharing {
                        Badge(variant = BadgeVariant::Destructive, class = "node-badge") {"LIVE"}
                    }
                    if move || role == Perm::Admin {
                        Badge(variant = BadgeVariant::Default, class = "node-badge") {"Admin"}
                    }
                    if move || role == Perm::Mod {
                        Badge(variant = BadgeVariant::Outline, class = "node-badge") {"Mod"}
                    }
                    spacer()
                    if move || muted {
                        Ico(svg = icons::MIC_MUTED, class = "flag flag-muted")
                    }
                    if move || deaf {
                        Ico(svg = icons::SPEAKER_MUTED, class = "flag flag-muted")
                    }
                    if move || locally_muted() {
                        Ico(svg = icons::SPEAKER_MUTED, class = "flag flag-local")
                    }
                }
            }
            {menu}
        }
    }
}
