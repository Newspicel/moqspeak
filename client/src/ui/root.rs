//! The window root: the theme it wears, the overlays every region shares, and the frame.

use std::time::Duration;

use zgui::prelude::*;
use zgui_ui::prelude::*;
use zgui_ui_tokens::prelude::*;

use crate::ui::frame::FrameProps;
use crate::ui::notices::NoticesProps;
use crate::ui::state::AppState;
use crate::ui::theme::MsTokensProps;

/// The theme provider, the theme's own properties, the tooltip clock, the toasts and the frame.
#[component]
pub fn Root() -> impl IntoView {
    let state = AppState::expect();
    let scheme = Signal::derive_local(move || state.scheme.get().color_scheme());
    let variant = Signal::derive_local(move || state.variant.get());

    view! {
        ThemeProvider(
            scheme = scheme,
            light = Signal::derive_local(move || variant.get().light()),
            dark = Signal::derive_local(move || variant.get().dark())
        ) {
            MsTokens(variant = variant, scheme = scheme)
            TooltipProvider(delay = Duration::from_millis(400), close_delay = Duration::ZERO) {
                Toaster(corner = ToastCorner::BottomRight, limit = 4, class = "ms-notice") {
                    Notices()
                    Frame()
                }
            }
        }
    }
}
