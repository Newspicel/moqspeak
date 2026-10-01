//! One client of the tree: the voice mark, the name, the tags and the marks of a muted voice.

use zgui::prelude::*;
use zgui::reactive::LocalStorage;
use zgui::vocab::SharedString;
use zgui_ui::prelude::*;

use crate::model::{ClientId, Role as Rank};
use crate::ui::parts::icons::{self, IconProps, IconSize};
use crate::ui::parts::{StatusDotProps, TagProps};
use crate::ui::screen::LiveTagProps;
use crate::ui::state::{AppState, ClientRow, Drag, Selection};
use crate::ui::tree::client_menu::ClientMenuProps;
use crate::ui::tree::view::{CLIENT_INSET, indent, px};

/// The tone of a client's mark: speaking, away or quiet.
pub fn voice_tone(talking: bool, away: bool) -> &'static str {
    if talking {
        "talk"
    } else if away {
        "warn"
    } else {
        "idle"
    }
}

/// A client row. A double press opens a private conversation. A press that moves far enough
/// drags the client onto another channel.
#[component]
pub fn ClientLine(id: ClientId, row: Signal<ClientRow, LocalStorage>) -> impl IntoView {
    let state = AppState::expect();
    let locally_muted = Signal::derive_local(move || state.local_mutes.with(|m| m.contains(&id)));
    let talking = move || state.talking.with(|t| t.contains(&id)) && !locally_muted.get();
    let tone = Signal::derive_local(move || voice_tone(talking(), row.with(|r| r.away)));
    let me = move || row.with(|r| r.me);
    // A press that started here and has not become a drag dies with this row.
    on_cleanup_local(move || {
        if state
            .drag
            .with_untracked(|d| d.as_ref().is_some_and(|d| d.client == id && !d.active))
        {
            state.drag.set(None);
        }
    });

    view! {
        ContextMenu {
            ContextMenuTrigger {
                row(
                    class = "ms-row ms-row--client",
                    attr:data-selected = move || (state.selected.get() == Selection::Client(id)).then(|| "true".to_owned()),
                    attr:data-away = move || row.with(|r| r.away).then(|| "true".to_owned()),
                    style:padding-left = move || px(indent(row.with(|r| r.depth)) + CLIENT_INSET),
                    tabindex = Focus::Sequential,
                    a11y:role = Role::TreeItem,
                    a11y:label = move || SharedString::from(row.with(|r| r.name.clone())),
                    on:click = move |ev| {
                        if ev.button == Some(PointerButton::Secondary) {
                            return;
                        }
                        if state.click(Selection::Client(id)) {
                            state.open_direct(id);
                        }
                    },
                    on:key_down = move |ev| {
                        if matches!(&ev.key, Key::Named(NamedKey::Enter)) {
                            state.open_direct(id);
                        }
                    },
                    on:pointer_down = move |ev| {
                        // Anyone can drag themselves; moving others takes a moderator.
                        if ev.button == Some(PointerButton::Primary)
                            && (me() || state.my_role_untracked() >= Rank::Mod)
                        {
                            state.drag.set(Some(Drag {
                                client: id,
                                name: row.with_untracked(|r| r.name.clone()),
                                origin: (f32::from(ev.position.x), f32::from(ev.position.y)),
                                active: false,
                            }));
                        }
                    }
                ) {
                    StatusDot(tone = tone)
                    text(class = "ms-row__name") {{move || row.with(|r| r.name.clone())}}
                    if move || me() {
                        Tag(text = "you")
                    }
                    if move || row.with(|r| r.role == Rank::Admin) {
                        Tag(text = "admin", tone = "primary")
                    }
                    if move || row.with(|r| r.role == Rank::Mod) {
                        Tag(text = "mod")
                    }
                    LiveTag(row = row)
                    box(class = "ms-row__gap")
                    row(class = "ms-row__flags") {
                        if move || row.with(|r| r.away) {
                            Icon(svg = icons::MOON, size = IconSize::Xs, class = "ms-row__flag")
                        }
                        if move || row.with(|r| r.muted) {
                            Icon(svg = icons::MIC_OFF, size = IconSize::Xs, class = "ms-row__flag")
                        }
                        if move || row.with(|r| r.deaf) {
                            Icon(svg = icons::HEADPHONE_OFF, size = IconSize::Xs, class = "ms-row__flag")
                        }
                        if move || locally_muted.get() {
                            Icon(svg = icons::VOLUME_X, size = IconSize::Xs, class = "ms-row__flag")
                        }
                    }
                }
            }
            ClientMenu(id = id, row = row)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::voice_tone;

    #[test]
    fn speaking_outranks_away() {
        assert_eq!(voice_tone(true, true), "talk");
        assert_eq!(voice_tone(false, true), "warn");
        assert_eq!(voice_tone(false, false), "idle");
    }
}
