//! One line of the log: when, a tone mark, and what.

use zgui::prelude::*;

use crate::ui::parts::StatusDotProps;
use crate::ui::state::LogLine;

/// One log line.
#[component]
pub fn LogEntry(line: LogLine) -> impl IntoView {
    let tone = line.tone.token();
    view! {
        row(class = "ms-log-line", attr:data-tone = tone) {
            text(class = "ms-log-line__time") {{line.time}}
            StatusDot(tone = Signal::stored_local(tone))
            text(class = "ms-log-line__text") {{line.text}}
        }
    }
}
