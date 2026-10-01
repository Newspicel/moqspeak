//! The chat and event log, its tabs, and the line to type in.

use zgui::prelude::*;

use crate::ui::IntoAny;
use zgui::reactive::RenderEffect;
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

/// One tab under the log.
#[component]
fn ChatTab(
    tab: Tab,
    svg: &'static str,
    title: Signal<String>,
    #[prop(default = false)] closable: bool,
) -> impl IntoView {
    let state = AppState::expect();
    let key = tab_key(&tab);
    let selected = {
        let tab = tab.clone();
        move || state.tab.with(|t| *t == tab)
    };
    let unread = move || state.unread.with(|u| u.contains(&key));
    let close_id = match &tab {
        Tab::Private(id) => Some(*id),
        _ => None,
    };
    view! {
        control(
            class = "chat-tab",
            class:selected = selected,
            class:unread = unread,
            tabindex = Focus::Sequential,
            a11y:role = Role::Tab,
            on:click = move |_| state.select_tab(tab.clone())
        ) {
            Ico(svg = svg)
            text {{move || title.get()}}
            if move || closable {
                control(
                    class = "chat-tab-close",
                    a11y:label = "Close tab",
                    on:click:stop = move |_| if let Some(id) = close_id { state.close_tab(id) }
                ) {"×"}
            }
        }
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
            row(class = "chat-tabs", a11y:role = Role::TabList) {
                ChatTab(tab = Tab::Server, svg = icons::SERVER, title = server_title)
                ChatTab(tab = Tab::Channel, svg = icons::CHANNEL, title = channel_title)
                for entry in move || state.tabs.get(), key = |t: &(u64, String)| t.clone() {
                    ChatTab(tab = Tab::Private(entry.0), svg = icons::CHAT, title = Signal::derive(move || entry.1.clone()), closable = true)
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
                Input(
                    value = draft,
                    placeholder = "Enter chat message…",
                    label = "Chat message",
                    class = "chat-input",
                    disabled = Signal::derive_local(move || !state.connected()),
                    on:key_down = move |ev| {
                        if matches!(&ev.key, Key::Named(NamedKey::Enter)) {
                            send();
                        }
                    }
                )
                control(
                    class = "btn btn-primary btn-send",
                    tabindex = Focus::Sequential,
                    a11y:label = "Send",
                    on:click = move |_| send()
                ) {"Send"}
            }
        }
    }
}
