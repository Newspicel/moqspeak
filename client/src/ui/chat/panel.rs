//! The chat column: the conversations as tabs, the messages of the one it shows, and the field
//! to write in. It opens to its width while it holds a conversation, and closes to nothing.

use std::rc::Rc;

use zgui::prelude::*;
use zgui::reactive::{LocalStorage, RenderEffect};
use zgui::view::{ScrollBehavior, ScrollTarget};
use zgui_ui::prelude::*;

use crate::model::ClientId;
use crate::ui::chat::composer::ComposerProps;
use crate::ui::chat::message::{MessageLineProps, starts_run};
use crate::ui::chat::tab::ChatTabProps;
use crate::ui::parts::icons;
use crate::ui::parts::{KeyProps, press};
use crate::ui::state::{AppState, Conversation, Message};

/// The column. It stays mounted, so opening and closing it moves its width and nothing else.
#[component]
pub fn ChatPanel(
    /// Whether the column stands open.
    #[prop(into)]
    open: Signal<bool, LocalStorage>,
) -> impl IntoView {
    let state = AppState::expect();
    let end = NodeRef::new();
    let shown = Memo::new(move |_| {
        state.chat.with(|c| {
            let messages = c.of(c.current);
            let mut lines = Vec::with_capacity(messages.len());
            let mut previous: Option<&Message> = None;
            for message in &messages {
                lines.push((message.clone(), starts_run(previous, message)));
                previous = Some(message);
            }
            lines
        })
    });

    // Follow the newest message.
    let follow = RenderEffect::new(move |_| {
        shown.track();
        if end.get().is_some() {
            end.scroll_to(ScrollTarget::IntoViewEnd, ScrollBehavior::Instant);
        }
    });
    on_cleanup_local(move || drop(follow));

    let channel_title = Signal::derive_local(move || {
        state
            .my_channel()
            .map(|c| format!("# {}", c.name))
            .unwrap_or_else(|| "Channel".to_owned())
    });
    let topic = move || {
        if state.chat.with(|c| c.current) != Conversation::Channel {
            return None;
        }
        state
            .my_channel()
            .map(|c| c.topic)
            .filter(|t| !t.is_empty())
    };
    let close = Rc::new(move || state.chat.update(|c| c.toggle()));

    view! {
        column(
            class = "ms-chat",
            attr:data-open = move || open.get().then(|| "true".to_owned()),
            a11y:hidden = move || !open.get()
        ) {
            column(class = "ms-chat__inner") {
                row(class = "ms-chat__head", on:pointer_down = press::move_window()) {
                    row(class = "ms-chat__tabs", a11y:role = Role::TabList, a11y:label = "Conversations") {
                        ChatTab(conversation = Conversation::Channel, title = channel_title)
                        ChatTab(conversation = Conversation::Server, title = Signal::stored_local("Server".to_owned()))
                        for entry in move || state.chat.with(|c| c.direct.clone()), key = |t: &(ClientId, String)| t.clone() {
                            ChatTab(conversation = Conversation::Private(entry.0), title = Signal::stored_local(entry.1.clone()))
                        }
                    }
                    Key(
                        svg = Signal::stored_local(icons::X),
                        label = Signal::stored_local("Close chat".to_owned()),
                        on_press = close
                    )
                }
                if move || topic().is_some() {
                    text(class = "ms-chat__topic") {{move || topic().unwrap_or_default()}}
                }
                if move || shown.with(Vec::is_empty) {
                    column(class = "ms-chat__empty") {"No messages yet"}
                } else {
                    ScrollArea(class = "ms-chat__scroll", label = "Messages") {
                        column(class = "ms-chat__log") {
                            for line in move || shown.get(), key = |l: &(Message, bool)| l.0.id {
                                MessageLine(message = line.0, head = line.1)
                            }
                            box(node_ref = end)
                        }
                    }
                }
                Composer()
            }
        }
    }
}
