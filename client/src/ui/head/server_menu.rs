//! The server key at the leading end of the head, and the menu it opens: what the server is, the
//! saved servers, and the ways to leave.

use zgui::prelude::*;
use zgui::reactive::LocalStorage;
use zgui_ui::prelude::*;

use crate::ui::head::tone::{connection_tone, voice_words};
use crate::ui::parts::icons::{self, IconProps, IconSize};
use crate::ui::parts::{Erase, StatusDotProps, act, press};
use crate::ui::state::{AppState, Bookmark, Modal};

/// What the menu says about the server, read when it opens.
fn facts(state: AppState) -> AnyView {
    let address = state.settings.with_untracked(|s| s.address.clone());
    let online = state.clients.with_untracked(Vec::len);
    let channels = state.channels.with_untracked(Vec::len);
    let voice = state.media.with_untracked(voice_words);
    let since = state
        .connected_at
        .get_untracked()
        .map(|at| format!(" · connected {}", short(at.elapsed().as_secs())))
        .unwrap_or_default();
    view! {
        column(class = "ms-server-facts") {
            text(class = "ms-server-facts__title") {{state.server_label()}}
            row(class = "ms-server-facts__line") {
                Icon(svg = icons::GLOBE, size = IconSize::Xs)
                text {{address}}
            }
            row(class = "ms-server-facts__line") {
                Icon(svg = icons::USERS, size = IconSize::Xs)
                text {{format!("{online} online · {channels} channels{since}")}}
            }
            row(class = "ms-server-facts__line") {
                Icon(svg = icons::AUDIO_LINES, size = IconSize::Xs)
                text {{voice}}
            }
        }
    }
    .any()
}

/// Puts the server address on the clipboard.
fn copy_address(state: AppState) {
    if let Some(clipboard) = try_use_clipboard() {
        let address = state.settings.with_untracked(|s| s.address.clone());
        clipboard.set_text(ClipboardKind::Standard, address);
    }
}

/// A duration in its largest unit.
fn short(secs: u64) -> String {
    match secs {
        0..=59 => "just now".into(),
        60..=3599 => format!("{}m ago", secs / 60),
        _ => format!("{}h {}m ago", secs / 3600, (secs % 3600) / 60),
    }
}

/// One saved server as a menu row.
fn bookmark_item(state: AppState, bookmark: Bookmark) -> AnyView {
    let here = state
        .settings
        .with_untracked(|s| s.address == bookmark.address)
        && state.online();
    let label = bookmark.label.clone();
    view! {
        MenuItem(on_select = act(move || state.connect_bookmark(&bookmark))) {
            row(class = "ms-menu-row") {
                StatusDot(tone = Signal::stored_local(if here { "ok" } else { "idle" }))
                text(class = "ms-menu-row__text") {{label.clone()}}
            }
        }
    }
    .any()
}

/// The server key and its menu.
#[component]
pub fn ServerMenu() -> impl IntoView {
    let state = AppState::expect();
    let tone: Signal<&'static str, LocalStorage> = Signal::derive_local(move || {
        state
            .status
            .with(|s| state.media.with(|m| connection_tone(s, m).0))
    });
    let busy = Signal::derive_local(move || {
        state
            .status
            .with(|s| state.media.with(|m| connection_tone(s, m).1))
    });
    let saved = move || {
        state
            .settings
            .with_untracked(|s| s.bookmark_of(&s.address).is_some())
    };

    view! {
        DropdownMenu {
            DropdownMenuTrigger(variant = ButtonVariant::Ghost, size = ButtonSize::Sm, class = "ms-server", on:pointer_down = press::hold()) {
                StatusDot(tone = tone, busy = busy)
                text(class = "ms-server__name") {{move || state.server_label()}}
                Icon(svg = icons::CHEVRON_DOWN, size = IconSize::Xs, class = "ms-server__chevron")
            }
            DropdownMenuContent {
                {facts(state)}
                MenuSeparator()
                MenuLabel {"Servers"}
                {state.settings.with_untracked(|s| s.bookmarks.clone()).into_iter().map(|b| bookmark_item(state, b)).collect::<Vec<_>>()}
                MenuSeparator()
                MenuItem(on_select = act(move || state.modal.set(Modal::Connect))) {
                    Icon(svg = icons::PLUG, size = IconSize::Sm)
                    text {"Connect to…"}
                }
                MenuItem(disabled = saved(), on_select = act(move || state.add_bookmark())) {
                    Icon(svg = icons::STAR, size = IconSize::Sm)
                    text {"Save server"}
                }
                MenuItem(on_select = act(move || copy_address(state))) {
                    Icon(svg = icons::COPY, size = IconSize::Sm)
                    text {"Copy address"}
                }
                MenuSeparator()
                MenuItem(destructive = true, on_select = act(move || state.disconnect())) {
                    Icon(svg = icons::UNPLUG, size = IconSize::Sm)
                    text {"Disconnect"}
                }
            }
        }
    }
}
