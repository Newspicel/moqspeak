//! One message of a conversation.

use zgui::prelude::*;

use crate::ui::parts::Erase;
use crate::ui::parts::icons::{self, IconProps, IconSize};
use crate::ui::state::{Message, MessageKind};

/// How long a writer's run of messages shares one head, in seconds.
const RUN: i64 = 300;

/// Whether `message` opens a new run: a new writer, a long pause, or a note.
pub fn starts_run(previous: Option<&Message>, message: &Message) -> bool {
    let Some(previous) = previous else {
        return true;
    };
    match (previous.author(), message.author()) {
        (Some(a), Some(b)) => a != b || (message.at - previous.at).num_seconds() > RUN,
        _ => true,
    }
}

/// A message. The first of a run carries the writer and the time. A note stands on its own.
#[component]
pub fn MessageLine(message: Message, head: bool) -> impl IntoView {
    let time = message.at.format("%H:%M").to_string();
    match message.kind {
        MessageKind::Text { from, own } => view! {
            column(
                class = "ms-msg",
                attr:data-head = head.then(|| "true".to_owned()),
                attr:data-own = own.then(|| "true".to_owned())
            ) {
                if move || head {
                    row(class = "ms-msg__head") {
                        text(class = "ms-msg__from") {{from.clone()}}
                        text(class = "ms-msg__time") {{time.clone()}}
                    }
                }
                text(class = "ms-msg__text") {{message.text}}
            }
        }
        .any(),
        MessageKind::Note => view! {
            row(class = "ms-msg ms-msg--note") {
                Icon(svg = icons::INFO, size = IconSize::Xs)
                text(class = "ms-msg__text") {{message.text}}
            }
        }
        .any(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::state::Conversation;

    fn text(id: u64, from: &str, secs: i64) -> Message {
        Message {
            id,
            conversation: Conversation::Channel,
            at: chrono::Local::now() + chrono::Duration::seconds(secs),
            kind: MessageKind::Text {
                from: from.into(),
                own: false,
            },
            text: String::new(),
        }
    }

    #[test]
    fn a_writer_keeps_one_head_until_a_pause_or_another_writer() {
        let a = text(1, "ann", 0);
        let b = text(2, "ann", 30);
        let c = text(3, "bob", 40);
        let d = text(4, "bob", 40 + RUN + 1);
        assert!(starts_run(None, &a));
        assert!(!starts_run(Some(&a), &b));
        assert!(starts_run(Some(&b), &c));
        assert!(starts_run(Some(&c), &d));
    }
}
