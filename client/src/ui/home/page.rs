//! The page the window shows while it holds no server: the name, the connect field and the
//! saved servers.

use zgui::prelude::*;
use zgui_ui::prelude::*;

use crate::ui::home::connect::ConnectFieldProps;
use crate::ui::home::servers::ServerListProps;

/// One centred column.
#[component]
pub fn Home() -> impl IntoView {
    view! {
        ScrollArea(class = "ms-home", label = "Connect") {
            column(class = "ms-home__center") {
            column(class = "ms-home__column") {
                column(class = "ms-home__brand") {
                    text(class = "ms-home__name") {"moqspeak"}
                    text(class = "ms-home__tagline") {"Voice over Media over QUIC"}
                }
                ConnectField()
                ServerList()
            }
            }
        }
    }
}
