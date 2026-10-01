//! One conversation in the chat head: its name, an unread mark, and for a private one a cross.

use zgui::prelude::*;
use zgui::reactive::{LocalStorage, RenderEffect};
use zgui::view::{ScrollBehavior, ScrollTarget};
use zgui::vocab::SharedString;

use crate::ui::parts::icons::{self, IconProps, IconSize};
use crate::ui::parts::press;
use crate::ui::state::{AppState, Conversation};

/// A ghost tab. A press shows its conversation.
#[component]
pub fn ChatTab(
    conversation: Conversation,
    #[prop(into)] title: Signal<String, LocalStorage>,
) -> impl IntoView {
    let state = AppState::expect();
    let active = move || state.chat.with(|c| c.current == conversation);
    let unread = move || state.chat.with(|c| c.unread.contains(&conversation));
    let closable = matches!(conversation, Conversation::Private(_));
    // The tab that shows its conversation stands in view of the scrolling row.
    let node = NodeRef::new();
    let reveal = RenderEffect::new(move |_| {
        if active() && node.get().is_some() {
            node.scroll_to(ScrollTarget::IntoView, ScrollBehavior::Instant);
        }
    });
    on_cleanup_local(move || drop(reveal));

    view! {
        control(
            class = "ms-chat-tab",
            node_ref = node,
            attr:data-active = move || active().then(|| "true".to_owned()),
            tabindex = Focus::Sequential,
            a11y:role = Role::Tab,
            a11y:label = move || SharedString::from(title.get()),
            a11y:selected = active,
            on:pointer_down = press::press(),
            on:click = move |_| state.chat.update(|c| c.show(conversation))
        ) {
            text(class = "ms-chat-tab__name") {{move || title.get()}}
            if move || unread() {
                box(class = "ms-chat-tab__unread", a11y:hidden = true)
            }
            if move || closable {
                control(
                    class = "ms-chat-tab__close",
                    a11y:role = Role::Button,
                    a11y:label = "Close conversation",
                    on:pointer_down = press::hold(),
                    on:click:stop = move |_| {
                        if let Conversation::Private(id) = conversation {
                            state.chat.update(|c| c.close_direct(id));
                        }
                    }
                ) {
                    Icon(svg = icons::X, size = IconSize::Xs)
                }
            }
        }
    }
}
