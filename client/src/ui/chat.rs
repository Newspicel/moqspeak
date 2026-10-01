//! The chat and event log, its tabs, and the line to type in.

use zgui::prelude::*;

use crate::ui::IntoAny;
use zgui::reactive::{RenderEffect, UnsyncCallback};
use zgui::view::{ScrollBehavior, ScrollTarget};
use zgui_ui::prelude::*;

use crate::ui::avatar::color_of;
use crate::ui::icons::{self, IcoProps};
use crate::ui::state::{AppState, Line, LineKind, Tab, tab_key};

#[component]
fn LogLine(line: Line) -> impl IntoView {
    let Line {
        time, kind, text, ..
    } = line;
    let time = format!("<{time}>");
    match kind {
        LineKind::Chat { from, .. } => view! {
            row(class = "line line-chat") {
                text(class = "line-time") {{time}}
                text(class = "line-from", style:color = Some(color_of(&from).to_owned())) {{format!("{from}:")}}
                text(class = "line-text") {{text}}
            }
        }
        .into_any(),
        LineKind::Poke { from } => view! {
            row(class = "line line-poke") {
                text(class = "line-time") {{time}}
                Ico(svg = icons::POKE)
                text(class = "line-text") {{format!("\"{from}\" poked you: {text}")}}
            }
        }
        .into_any(),
        LineKind::Error => view! {
            row(class = "line line-error") {
                text(class = "line-time") {{time}}
                Ico(svg = icons::ERROR)
                text(class = "line-text") {{text}}
            }
        }
        .into_any(),
        LineKind::Info => view! {
            row(class = "line line-info") {
                text(class = "line-time") {{time}}
                Ico(svg = icons::INFO)
                text(class = "line-text") {{text}}
            }
        }
        .into_any(),
        LineKind::Event => view! {
            row(class = "line line-event") {
                text(class = "line-time") {{time}}
                text(class = "line-text") {{text}}
            }
        }
        .into_any(),
    }
}

/// One chat tab trigger: an icon, the title and an unread dot. Private tabs close from their
/// context menu, as in TeamSpeak.
#[component]
fn ChatTab(tab: Tab, svg: &'static str, title: Signal<String>) -> impl IntoView {
    let state = AppState::expect();
    let key = tab_key(&tab);
    let unread = {
        let key = key.clone();
        Signal::derive(move || state.unread.with(|u| u.contains(&key)))
    };
    let trigger = {
        let key = key.clone();
        move || {
            view! {
                TabsTrigger(value = key.clone(), class = "chat-tab") {
                    Ico(svg = svg)
                    text {{move || title.get()}}
                    if move || unread.get() {
                        box(class = "unread-dot") {}
                    }
                }
            }
        }
    };
    match tab {
        Tab::Private(id) => view! {
            ContextMenu {
                ContextMenuTrigger { {trigger()} }
                ContextMenuContent {
                    MenuItem(on_select = Some(UnsyncCallback::new(move |_: ()| state.close_tab(id)))) {"Close Tab"}
                }
            }
        }
        .into_any(),
        _ => trigger().into_any(),
    }
}

#[component]
pub fn ChatPanel() -> impl IntoView {
    let state = AppState::expect();
    let draft = RwSignal::new_local(String::new());
    let end = NodeRef::new();

    let visible = Memo::new(move |_| {
        let tab = state.tab.get();
        state.lines.with(|lines| {
            lines
                .iter()
                .filter(|l| l.tab == tab)
                .cloned()
                .collect::<Vec<_>>()
        })
    });

    // Follow the newest line.
    let follow = RenderEffect::new(move |_| {
        visible.track();
        if let Some(node) = end.get() {
            let _ = node;
            end.scroll_to(ScrollTarget::IntoViewEnd, ScrollBehavior::Instant);
        }
    });
    on_cleanup_local(move || drop(follow));

    // The tab strip speaks strings; the state speaks tabs.
    let current = Binding::controlled(
        Signal::derive_local(move || tab_key(&state.tab.get())),
        move |key: String| {
            let tab = match key.as_str() {
                "server" => Tab::Server,
                "channel" => Tab::Channel,
                other => match other.strip_prefix('p').and_then(|id| id.parse().ok()) {
                    Some(id) => Tab::Private(id),
                    None => return,
                },
            };
            state.select_tab(tab);
        },
    );

    let send = move || {
        let text = draft.get_untracked();
        if text.trim().is_empty() {
            return;
        }
        state.send_chat(text);
        draft.set(String::new());
    };

    let channel_title = Signal::derive(move || {
        state
            .my_client()
            .and_then(|c| state.channel(c.channel))
            .map(|c| c.name)
            .unwrap_or_else(|| "Channel".to_owned())
    });
    let server_title = Signal::derive(move || {
        let name = state.server.with(|s| s.name.clone());
        if name.is_empty() {
            "Server".to_owned()
        } else {
            name
        }
    });

    view! {
        column(class = "pane chat-pane") {
            Tabs(value = current, label = "Chat", class = "chat-tabs") {
                TabsList(variant = TabsListVariant::Line) {
                    ChatTab(tab = Tab::Server, svg = icons::SERVER, title = server_title)
                    ChatTab(tab = Tab::Channel, svg = icons::CHANNEL, title = channel_title)
                    for entry in move || state.tabs.get(), key = |t: &(u64, String)| t.clone() {
                        ChatTab(tab = Tab::Private(entry.0), svg = icons::CHAT, title = Signal::derive(move || entry.1.clone()))
                    }
                }
            }
            ScrollArea(class = "chat-scroll", label = "Chat log") {
                column(class = "chat-log") {
                    for line in move || visible.get(), key = |l: &Line| l.id {
                        LogLine(line = line)
                    }
                    box(class = "chat-end", node_ref = end) {}
                }
            }
            row(class = "chat-input-row") {
                InputGroup(class = "chat-input", disabled = Signal::derive_local(move || !state.connected())) {
                    InputGroupInput(
                        value = draft,
                        placeholder = "Enter chat message…",
                        disabled = Signal::derive_local(move || !state.connected()),
                        a11y:label = "Chat message",
                        on:key_down = move |ev| {
                            if matches!(&ev.key, Key::Named(NamedKey::Enter)) {
                                send();
                            }
                        }
                    )
                    InputGroupAddon(align = InputGroupAddonAlign::InlineEnd) {
                        InputGroupButton(variant = ButtonVariant::Default, size = InputGroupButtonSize::Sm, on:click = move |_| send()) {"Send"}
                    }
                }
            }
        }
    }
}
