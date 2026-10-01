//! Starting and stopping a screen share from the window.

use zgui::prelude::*;

use crate::engine::Command;
use crate::screen::Source;
use crate::ui::state::{AppState, Modal};

/// Starts or stops sharing. With several monitors, asks which one first.
pub fn toggle_share(state: AppState) {
    if state.sharing.get_untracked() {
        state.send(Command::StopShare);
        return;
    }
    #[cfg(target_os = "macos")]
    if crate::screen::mac::picker_available() {
        // The system picker chooses a window, an app or a display. Its answer arrives on
        // another thread and comes back to this one through the UI handle.
        let ui = ui();
        crate::screen::mac::pick(move |outcome| {
            ui.post(move || match outcome {
                Ok(Some(picked)) => state.send(Command::StartShare(Source::Picked(
                    crate::screen::Handoff::new(picked),
                ))),
                Ok(None) => {}
                Err(e) => state.fail("Screen share failed", Some(format!("{e:#}"))),
            });
        });
        return;
    }
    let monitors = crate::screen::list_monitors();
    match monitors.len() {
        0 => state.fail(
            "No screen to share",
            Some("On macOS, allow moqspeak under System Settings → Privacy & Security → Screen Recording.".into()),
        ),
        1 => state.send(Command::StartShare(Source::Monitor(monitors[0].id))),
        _ => state.modal.set(Modal::Share { monitors }),
    }
}
