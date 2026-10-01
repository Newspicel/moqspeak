//! The log dock at the foot of the server column: what happened, newest last.

use std::rc::Rc;

use zgui::prelude::*;
use zgui::reactive::RenderEffect;
use zgui::view::{ScrollBehavior, ScrollTarget};
use zgui_ui::prelude::*;

use crate::ui::log::line::LogEntryProps;
use crate::ui::parts::KeyProps;
use crate::ui::parts::icons;
use crate::ui::state::{AppState, LogLine};

/// The strip with its keys, and the lines under it.
#[component]
pub fn LogDock() -> impl IntoView {
    let state = AppState::expect();
    let end = NodeRef::new();
    let lines = Memo::new(move |_| state.log.with(|l| l.lines.clone()));
    let follow = RenderEffect::new(move |_| {
        lines.track();
        if end.get().is_some() {
            end.scroll_to(ScrollTarget::IntoViewEnd, ScrollBehavior::Instant);
        }
    });
    on_cleanup_local(move || drop(follow));
    let clear = Rc::new(move || state.log.update(|l| l.clear()));
    let close = Rc::new(move || state.log_open.set(false));

    view! {
        column(class = "ms-log", a11y:role = Role::Log, a11y:label = "Server log") {
            row(class = "ms-log__strip") {
                text(class = "ms-log__title") {"Log"}
                text(class = "ms-log__count") {{move || lines.with(Vec::len).to_string()}}
                box(class = "ms-log__gap")
                Key(
                    svg = Signal::stored_local(icons::ERASER),
                    label = Signal::stored_local("Clear log".to_owned()),
                    on_press = clear
                )
                Key(
                    svg = Signal::stored_local(icons::X),
                    label = Signal::stored_local("Hide log".to_owned()),
                    on_press = close
                )
            }
            ScrollArea(class = "ms-log__scroll", label = "Log lines") {
                column(class = "ms-log__lines") {
                    if move || lines.with(Vec::is_empty) {
                        text(class = "ms-log__empty") {"Nothing yet"}
                    }
                    for line in move || lines.get(), key = |l: &LogLine| l.id {
                        LogEntry(line = line)
                    }
                    box(node_ref = end)
                }
            }
        }
    }
}
