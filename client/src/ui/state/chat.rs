//! The conversations: their messages, which one shows, which ones wait unread, and whether the
//! chat panel stands open.
//!
//! The panel opens on its own when a message arrives while it is closed. A person who closes it
//! keeps it closed: later messages only mark their conversation unread until the panel opens
//! again.

use std::collections::BTreeSet;

use crate::model::ClientId;

/// One conversation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Conversation {
    /// The channel you are in.
    Channel,
    /// Everyone on the server.
    Server,
    /// One other client.
    Private(ClientId),
}

/// What one message is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MessageKind {
    /// Text somebody wrote.
    Text { from: String, own: bool },
    /// A note from the server, such as its welcome.
    Note,
}

/// One message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Message {
    pub id: u64,
    pub conversation: Conversation,
    /// When it arrived.
    pub at: chrono::DateTime<chrono::Local>,
    pub kind: MessageKind,
    pub text: String,
}

impl Message {
    /// Who wrote it, for a text.
    pub fn author(&self) -> Option<&str> {
        match &self.kind {
            MessageKind::Text { from, .. } => Some(from),
            _ => None,
        }
    }
}

/// How many messages are kept.
const KEEP: usize = 2000;

/// Every conversation and the state of the panel that shows them.
#[derive(Clone, Debug, PartialEq)]
pub struct Chat {
    pub messages: Vec<Message>,
    /// The private conversations, with the name of the other client.
    pub direct: Vec<(ClientId, String)>,
    pub current: Conversation,
    pub unread: BTreeSet<Conversation>,
    pub open: bool,
    /// Whether the person closed the panel, which keeps it from opening on its own.
    pub dismissed: bool,
    next: u64,
}

impl Default for Chat {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
            direct: Vec::new(),
            current: Conversation::Channel,
            unread: BTreeSet::new(),
            open: false,
            dismissed: false,
            next: 1,
        }
    }
}

impl Chat {
    /// Adds a message. A message from somebody else opens the panel, or marks its conversation
    /// unread when the panel is closed by hand or shows another conversation.
    pub fn push(&mut self, conversation: Conversation, kind: MessageKind, text: String) {
        let own = matches!(kind, MessageKind::Text { own: true, .. });
        let quiet = matches!(kind, MessageKind::Note);
        self.messages.push(Message {
            id: self.next,
            conversation,
            at: chrono::Local::now(),
            kind,
            text,
        });
        self.next += 1;
        if self.messages.len() > KEEP {
            self.messages.drain(..KEEP / 4);
        }
        if own || quiet {
            return;
        }
        if !self.open && !self.dismissed {
            self.open = true;
            self.current = conversation;
        }
        if !self.open || self.current != conversation {
            self.unread.insert(conversation);
        }
    }

    /// Records a private conversation with `id`.
    pub fn add_direct(&mut self, id: ClientId, name: String) {
        if !self.direct.iter().any(|(t, _)| *t == id) {
            self.direct.push((id, name));
        }
    }

    /// Opens the private conversation with `id`, and the panel with it.
    pub fn open_direct(&mut self, id: ClientId, name: String) {
        self.add_direct(id, name);
        self.show(Conversation::Private(id));
    }

    /// Shows `conversation` in an open panel.
    pub fn show(&mut self, conversation: Conversation) {
        self.current = conversation;
        self.open = true;
        self.dismissed = false;
        self.unread.remove(&conversation);
    }

    /// Opens a closed panel, or closes an open one by hand.
    pub fn toggle(&mut self) {
        if self.open {
            self.open = false;
            self.dismissed = true;
        } else {
            self.show(self.current);
        }
    }

    /// Forgets the private conversation with `id`.
    pub fn close_direct(&mut self, id: ClientId) {
        self.direct.retain(|(t, _)| *t != id);
        self.unread.remove(&Conversation::Private(id));
        if self.current == Conversation::Private(id) {
            self.current = Conversation::Channel;
        }
    }

    /// The messages of `conversation`, oldest first.
    pub fn of(&self, conversation: Conversation) -> Vec<Message> {
        self.messages
            .iter()
            .filter(|m| m.conversation == conversation)
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(from: &str, own: bool) -> MessageKind {
        MessageKind::Text {
            from: from.into(),
            own,
        }
    }

    #[test]
    fn a_first_message_opens_the_panel_on_its_conversation() {
        let mut chat = Chat::default();
        chat.push(Conversation::Server, text("bob", false), "hi".into());
        assert!(chat.open);
        assert_eq!(chat.current, Conversation::Server);
        assert!(chat.unread.is_empty());
    }

    #[test]
    fn a_panel_closed_by_hand_stays_closed_and_counts_unread() {
        let mut chat = Chat::default();
        chat.push(Conversation::Channel, text("bob", false), "hi".into());
        chat.toggle();
        chat.push(Conversation::Channel, text("bob", false), "again".into());
        assert!(!chat.open);
        assert!(chat.unread.contains(&Conversation::Channel));
        chat.toggle();
        assert!(chat.open);
        assert!(chat.unread.is_empty());
    }

    #[test]
    fn a_message_in_another_conversation_waits_unread() {
        let mut chat = Chat::default();
        chat.show(Conversation::Channel);
        chat.push(Conversation::Private(7), text("ann", false), "psst".into());
        assert_eq!(chat.current, Conversation::Channel);
        assert!(chat.unread.contains(&Conversation::Private(7)));
    }

    #[test]
    fn own_messages_and_notes_leave_the_panel_alone() {
        let mut chat = Chat::default();
        chat.push(Conversation::Channel, text("me", true), "hello".into());
        chat.push(Conversation::Server, MessageKind::Note, "welcome".into());
        assert!(!chat.open);
        assert!(chat.unread.is_empty());
    }

    #[test]
    fn closing_a_direct_conversation_returns_to_the_channel() {
        let mut chat = Chat::default();
        chat.open_direct(3, "ann".into());
        assert_eq!(chat.current, Conversation::Private(3));
        chat.close_direct(3);
        assert_eq!(chat.current, Conversation::Channel);
        assert!(chat.direct.is_empty());
    }
}
