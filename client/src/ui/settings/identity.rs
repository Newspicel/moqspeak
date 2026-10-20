//! The TeamSpeak identity: what a server knows this client as, how high its security level
//! stands, and reading one in from the TeamSpeak client.

use std::time::Duration;

use zgui::prelude::*;
use zgui::reactive::StoredValue;
use zgui_ui::prelude::*;

use crate::transport::ts3::identity::{self, Imported, Progress, Search};
use crate::ui::state::{AppState, Level, Note};

/// How often a running search reports what it reached.
const POLL: Duration = Duration::from_millis(120);

/// A count of offsets, short enough for a row.
fn tally(tried: u64) -> String {
    match tried {
        n if n >= 1_000_000_000 => format!("{:.1}B", n as f64 / 1e9),
        n if n >= 1_000_000 => format!("{:.1}M", n as f64 / 1e6),
        n if n >= 1_000 => format!("{:.1}k", n as f64 / 1e3),
        n => n.to_string(),
    }
}

/// What the pane says about an identity waiting to be taken.
fn preview(imported: &Imported) -> String {
    let uid = ts3proto::get_uid_from_public_key(imported.identity.public_key_base64());
    let level = imported.identity.security_level();
    let mut name = imported
        .label
        .clone()
        .unwrap_or_else(|| "Identity".to_owned());
    if let Some(nickname) = &imported.nickname {
        name = format!("{name} ({nickname})");
    }
    format!("{name} · level {level} · {uid}")
}

/// The identity pane.
#[component]
pub fn IdentityPane() -> impl IntoView {
    let state = AppState::expect();
    let saved = RwSignal::new_local(identity::saved());
    let target = RwSignal::new_local(
        saved
            .with_untracked(|s| s.as_ref().map_or(8, |id| id.security_level() + 1))
            .to_string(),
    );
    let pasted = RwSignal::new_local(String::new());
    let pending = RwSignal::new_local(None::<Imported>);
    let trouble = RwSignal::new_local(None::<String>);
    let progress = RwSignal::new_local(None::<Progress>);
    let search = StoredValue::new_local(None::<Search>);

    let uid = Signal::derive_local(move || {
        saved.with(|s| {
            s.as_ref().map_or_else(
                || "none yet".to_owned(),
                |id| ts3proto::get_uid_from_public_key(id.public_key_base64()),
            )
        })
    });
    let level = Signal::derive_local(move || {
        saved.with(|s| {
            s.as_ref().map_or_else(
                || "none yet".to_owned(),
                |id| id.security_level().to_string(),
            )
        })
    });
    let running =
        Signal::derive_local(move || progress.with(|p| p.as_ref().is_some_and(|p| p.running)));
    let empty = Signal::derive_local(move || saved.with(Option::is_none));

    // A search reports what it reached, and what it found is kept once it ends.
    let tick = set_interval(POLL, move || {
        let Some(now) = search.with_value(|s| s.as_ref().map(Search::progress)) else {
            return;
        };
        if now.running {
            progress.set(Some(now));
            return;
        }
        let found = search.with_value(|s| s.as_ref().and_then(Search::best));
        search.set_value(None);
        progress.set(None);
        let Some(better) = found else {
            trouble.set(Some("The search ended with nothing better.".to_owned()));
            return;
        };
        let reached = better.security_level();
        match identity::replace(&better) {
            Ok(()) => {
                saved.set(Some(better));
                target.set((reached + 1).to_string());
                state.note(Note {
                    level: Level::Info,
                    title: format!("Security level {reached}"),
                    detail: Some("The new level counts the next time you connect.".to_owned()),
                    reply: None,
                });
            }
            Err(e) => trouble.set(Some(e.to_string())),
        }
    });
    on_cleanup_local(move || {
        search.with_value(|s| {
            if let Some(search) = s {
                search.stop();
            }
        });
        if let Some(found) = search.with_value(|s| s.as_ref().and_then(Search::best)) {
            let _ = identity::replace(&found);
        }
        drop(tick);
    });

    let raise = move || {
        trouble.set(None);
        let Some(current) = saved.with(|s| s.clone()) else {
            trouble.set(Some(
                "There is no identity yet. Import one, or connect to a TeamSpeak server once."
                    .to_owned(),
            ));
            return;
        };
        let Ok(want) = target.with(|t| t.trim().parse::<i32>()) else {
            trouble.set(Some("The level is a whole number.".to_owned()));
            return;
        };
        let have = current.security_level();
        if want <= have {
            trouble.set(Some(format!(
                "The identity already stands at level {have}."
            )));
            return;
        }
        let started = Search::start(current, want);
        progress.set(Some(started.progress()));
        search.set_value(Some(started));
    };

    let halt = move || {
        search.with_value(|s| {
            if let Some(search) = s {
                search.stop();
            }
        });
    };

    let send_out = move || {
        let Some(current) = saved.with(|s| s.clone()) else {
            return;
        };
        let nickname = state.settings.with_untracked(|s| s.nickname.clone());
        match identity::to_downloads(&current, &nickname) {
            Ok(path) => state.note(Note {
                level: Level::Info,
                title: "Identity exported".to_owned(),
                detail: Some(path.display().to_string()),
                reply: None,
            }),
            Err(e) => state.note(Note {
                level: Level::Error,
                title: "The identity did not export".to_owned(),
                detail: Some(e.to_string()),
                reply: None,
            }),
        }
    };

    let read = move || {
        trouble.set(None);
        match identity::from_input(&pasted.with(|t| t.clone())) {
            Ok(imported) => pending.set(Some(imported)),
            Err(e) => {
                pending.set(None);
                trouble.set(Some(e.to_string()));
            }
        }
    };

    let take = move || {
        let Some(incoming) = pending.with(|p| p.as_ref().map(|i| i.identity.clone())) else {
            return;
        };
        match identity::replace(&incoming) {
            Ok(()) => {
                let uid = ts3proto::get_uid_from_public_key(incoming.public_key_base64());
                let reached = incoming.security_level();
                saved.set(Some(incoming));
                target.set((reached + 1).to_string());
                pending.set(None);
                pasted.set(String::new());
                trouble.set(None);
                state.note(Note {
                    level: Level::Info,
                    title: "Identity imported".to_owned(),
                    detail: Some(format!("{uid} counts the next time you connect.")),
                    reply: None,
                });
            }
            Err(e) => trouble.set(Some(e.to_string())),
        }
    };

    view! {
        SettingsPane(value = "identity") {
            SettingsGroup {
                SettingsGroupLabel {"TeamSpeak identity"}
                SettingsGroupDescription {"A TeamSpeak server knows this client by its identity, and keeps the groups it granted it."}
                SettingsItem(label = "Unique id", description = "What a server shows this client as.") {
                    text(class = "ms-identity__uid") {{move || uid.get()}}
                }
                SettingsItem(label = "Security level", description = "Proof of work over the identity. A server may ask for a minimum.") {
                    text(class = "ms-identity__level") {{move || level.get()}}
                }
                SettingsItem(label = "Export", description = "Write the identity to a file the TeamSpeak client imports under Identities.") {
                    Button(variant = ButtonVariant::Outline, disabled = empty, on:click = move |_| send_out()) {"Export"}
                }
            }
            SettingsGroup {
                SettingsGroupLabel {"Raise the security level"}
                SettingsGroupDescription {"The search runs on every core. A level costs twice the one before it, and leaving this page keeps the best offset found."}
                SettingsItem(label = "Target level") {
                    row(class = "ms-identity__row") {
                        Input(value = target, label = "Target security level", disabled = running, class = "ms-identity__target")
                        if move || running.get() {
                            Button(variant = ButtonVariant::Outline, on:click = move |_| halt()) {"Stop"}
                        }
                        if move || !running.get() {
                            Button(on:click = move |_| raise()) {"Raise"}
                        }
                    }
                }
                if move || running.get() {
                    text(class = "ms-identity__progress") {
                        {move || progress.with(|p| p.as_ref().map_or_else(String::new, |p| {
                            format!("{} offsets tried · best level {}", tally(p.tried), p.level)
                        }))}
                    }
                }
            }
            SettingsGroup {
                SettingsGroupLabel {"Import an identity"}
                SettingsGroupDescription {"Paste what the TeamSpeak client exports: the file, a path to it, or the key and its counter."}
                column(class = "ms-identity__import") {
                    Textarea(
                        value = pasted,
                        label = "Identity to import",
                        placeholder = "The exported file, a path to it, or key:counter",
                        class = "ms-identity__paste"
                    )
                    row(class = "ms-identity__row") {
                        Button(variant = ButtonVariant::Outline, on:click = move |_| read()) {"Read"}
                    }
                    if move || pending.with(|p| p.is_some()) {
                        column(class = "ms-identity__preview") {
                            text {{move || pending.with(|p| p.as_ref().map_or_else(String::new, preview))}}
                            text(class = "ms-identity__hint") {"The identity in use is kept beside the new one, as teamspeak.identity.bak."}
                            row(class = "ms-identity__row") {
                                Button(on:click = move |_| take()) {"Replace my identity"}
                                Button(variant = ButtonVariant::Ghost, on:click = move |_| pending.set(None)) {"Cancel"}
                            }
                        }
                    }
                    if move || trouble.with(|t| t.is_some()) {
                        text(class = "ms-identity__trouble") {{move || trouble.with(|t| t.clone().unwrap_or_default())}}
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shortens_a_count_of_offsets() {
        assert_eq!(tally(0), "0");
        assert_eq!(tally(512), "512");
        assert_eq!(tally(4_096), "4.1k");
        assert_eq!(tally(12_400_000), "12.4M");
        assert_eq!(tally(2_500_000_000), "2.5B");
    }

    #[test]
    fn says_what_an_identity_holds() {
        let identity = ts3proto::generate_identity(0);
        let uid = ts3proto::get_uid_from_public_key(identity.public_key_base64());
        let level = identity.security_level();
        let line = preview(&Imported {
            identity: identity.clone(),
            label: Some("Main".into()),
            nickname: Some("Julian".into()),
        });
        assert_eq!(line, format!("Main (Julian) · level {level} · {uid}"));
    }

    #[test]
    fn names_an_identity_that_carries_no_name() {
        let identity = ts3proto::generate_identity(0);
        let line = preview(&Imported {
            identity,
            label: None,
            nickname: None,
        });
        assert!(line.starts_with("Identity · level "), "{line}");
    }
}
