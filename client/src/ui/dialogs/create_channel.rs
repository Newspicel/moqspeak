//! Create a channel, at the top or under another one.

use zgui::prelude::*;
use zgui::reactive::StoredValue;
use zgui_ui::prelude::*;

use crate::model::ClientMsg;
use crate::ui::dialogs::host::FormRowProps;
use crate::ui::state::{AppState, Modal};

/// The name, topic, description and size of the new channel.
#[component]
pub fn CreateChannelBody() -> impl IntoView {
    let state = AppState::expect();
    let parent = match state.modal.get_untracked() {
        Modal::CreateChannel { parent } => parent,
        _ => None,
    };
    let name = RwSignal::new_local(String::new());
    let topic = RwSignal::new_local(String::new());
    let description = RwSignal::new_local(String::new());
    let max = RwSignal::new_local(String::new());
    let subtitle = StoredValue::new(match parent.and_then(|p| state.channel(p)) {
        Some(p) => format!("Inside {}.", p.name),
        None => "At the top of the tree.".to_owned(),
    });
    let create = move || {
        let n = name.get_untracked().trim().to_owned();
        if n.is_empty() {
            return;
        }
        state.msg(ClientMsg::CreateChannel {
            name: n,
            topic: topic.get_untracked(),
            description: description.get_untracked(),
            parent,
            max_clients: max.get_untracked().trim().parse().unwrap_or(0),
        });
        state.modal.set(Modal::None);
    };
    view! {
        DialogHeader {
            DialogTitle {"New channel"}
            DialogDescription {{subtitle.get_value()}}
        }
        column(class = "ms-form") {
            FormRow(label = "Name") { Input(value = name, label = "Channel name") }
            FormRow(label = "Topic") { Input(value = topic, label = "Topic") }
            FormRow(label = "Description") { Textarea(value = description, label = "Description") }
            FormRow(label = "Most clients") { Input(value = max, label = "Most clients", placeholder = "No limit") }
        }
        DialogFooter {
            DialogClose(variant = ButtonVariant::Outline) {"Cancel"}
            Button(on:click = move |_| create()) {"Create"}
        }
    }
}
