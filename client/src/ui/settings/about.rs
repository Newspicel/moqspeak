//! What moqspeak is.

use zgui::prelude::*;
use zgui_ui::prelude::*;

/// The about pane.
#[component]
pub fn AboutPane() -> impl IntoView {
    view! {
        SettingsPane(value = "about") {
            SettingsGroup {
                SettingsGroupLabel {"moqspeak"}
                SettingsGroupDescription {{format!("Version {}", env!("CARGO_PKG_VERSION"))}}
                column(class = "ms-about") {
                    text {"A voice client written in Rust with zgui."}
                    text {"Control runs on a Cloudflare Worker with one Durable Object per server."}
                    text {"Voice is Opus over Media over QUIC through Cloudflare's relay."}
                }
            }
        }
    }
}
