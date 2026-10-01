//! The menu bar: Connections, Bookmarks, Self, Tools, Help.

use std::sync::atomic::Ordering;

use zgui::prelude::*;
use zgui::reactive::UnsyncCallback;
use zgui_ui::prelude::*;

use crate::engine::ConnStatus;
use crate::ui::state::{AppState, Bookmark, Modal};

fn act(f: impl Fn() + 'static) -> Option<UnsyncCallback<()>> {
    Some(UnsyncCallback::new(move |_: ()| f()))
}

#[component]
pub fn MainMenu() -> impl IntoView {
    let state = AppState::expect();
    let online = move || {
        state
            .status
            .with(|s| *s != ConnStatus::Disconnected && !matches!(s, ConnStatus::Failed(_)))
    };
    let offline = Signal::derive_local(move || !online());
    let not_connected = Signal::derive_local(move || !state.connected());
    let loopback = RwSignal::new_local(false);

    let bookmarks = move || state.settings.with(|s| s.bookmarks.clone());

    view! {
        Menubar(label = "Main menu", class = "menubar") {
            MenubarMenu(value = "connections") {
                MenubarTrigger {"Connections"}
                MenubarContent {
                    MenubarItem(
                        shortcut = "⌘S",
                        on_select = act(move || state.modal.set(Modal::Connect))
                    ) {"Connect…"}
                    MenubarItem(
                        disabled = offline,
                        on_select = act(move || state.disconnect())
                    ) {"Disconnect"}
                    MenubarSeparator()
                    MenubarItem(shortcut = "⌘Q", on_select = act(|| std::process::exit(0))) {"Quit"}
                }
            }
            MenubarMenu(value = "bookmarks") {
                MenubarTrigger {"Bookmarks"}
                MenubarContent {
                    MenubarItem(
                        disabled = not_connected,
                        on_select = act(move || {
                            let server = state.server.get_untracked();
                            let settings = state.settings.get_untracked();
                            state.update_settings(|s| {
                                if !s.bookmarks.iter().any(|b| b.address == settings.address) {
                                    s.bookmarks.push(Bookmark {
                                        label: server.name.clone(),
                                        address: settings.address.clone(),
                                        nickname: settings.nickname.clone(),
                                    });
                                }
                            });
                            state.info(format!("Added \"{}\" to bookmarks", server.name));
                        })
                    ) {"Add to Bookmarks"}
                    MenubarSeparator()
                    for bookmark in move || bookmarks(), key = |b: &Bookmark| b.clone() {
                        MenubarItem(on_select = {
                            let b = bookmark.clone();
                            act(move || {
                                let nickname = if b.nickname.is_empty() {
                                    state.settings.with_untracked(|s| s.nickname.clone())
                                } else {
                                    b.nickname.clone()
                                };
                                state.connect(b.address.clone(), nickname);
                            })
                        }) {{bookmark.label.clone()}}
                    }
                }
            }
            MenubarMenu(value = "self") {
                MenubarTrigger {"Self"}
                MenubarContent {
                    MenubarItem(on_select = act(move || state.set_away(!state.away.get_untracked()))) {
                        {move || if state.away.get() { "Back from away" } else { "Set away" }}
                    }
                    MenubarSeparator()
                    MenubarItem(
                        shortcut = "⌘M",
                        on_select = act(move || state.set_mic_muted(!state.mic_muted.get_untracked()))
                    ) {
                        {move || if state.mic_muted.get() { "Unmute microphone" } else { "Mute microphone" }}
                    }
                    MenubarItem(
                        on_select = act(move || state.set_deafened(!state.deafened.get_untracked()))
                    ) {
                        {move || if state.deafened.get() { "Unmute speakers" } else { "Mute speakers" }}
                    }
                }
            }
            MenubarMenu(value = "tools") {
                MenubarTrigger {"Tools"}
                MenubarContent {
                    MenubarItem(
                        shortcut = "⌘P",
                        on_select = act(move || state.modal.set(Modal::Options))
                    ) {"Options…"}
                    MenubarItem(on_select = act(move || {
                        let on = !loopback.get_untracked();
                        loopback.set(on);
                        state.engine.with_value(|e| e.audio.shared.loopback.store(on, Ordering::Relaxed));
                        state.info(if on {
                            "Microphone test on: you now hear yourself"
                        } else {
                            "Microphone test off"
                        });
                    })) {
                        {move || if loopback.get() { "Stop microphone test" } else { "Microphone test" }}
                    }
                }
            }
            MenubarMenu(value = "help") {
                MenubarTrigger {"Help"}
                MenubarContent {
                    MenubarItem(on_select = act(move || state.modal.set(Modal::About))) {"About moqspeak"}
                }
            }
        }
    }
}
