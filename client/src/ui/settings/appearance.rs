//! How the window looks: the theme and the surface.

use zgui::prelude::*;
use zgui_ui::prelude::*;

use crate::ui::parts::Erase;
use crate::ui::settings::theme_card::ThemeCardProps;
use crate::ui::state::AppState;
use crate::ui::theme::{Scheme, Variant};

/// The appearance pane.
#[component]
pub fn AppearancePane() -> impl IntoView {
    let state = AppState::expect();
    let scheme = Binding::controlled(
        Signal::derive_local(move || vec![state.scheme.get().name().to_owned()]),
        move |v: Vec<String>| {
            if let Some(s) = v.first() {
                state.set_scheme(Scheme::parse(s));
            }
        },
    );

    view! {
        SettingsPane(value = "appearance") {
            SettingsGroup {
                SettingsGroupLabel {"Theme"}
                SettingsGroupDescription {"The palette the window wears, at a light and at a dark surface."}
                row(class = "ms-themes", a11y:role = Role::RadioGroup, a11y:label = "Theme") {
                    {Variant::ALL.iter().map(|variant| view! { ThemeCard(variant = *variant) }.any()).collect::<Vec<_>>()}
                }
                SettingsItem(label = "Surface", description = "System follows the desktop.") {
                    ToggleGroup(value = scheme, spacing = 2.0, label = "Surface", class = "ms-switch") {
                        {Scheme::ALL.iter().map(|s| view! { ToggleGroupItem(value = s.name()) {{s.label()}} }.any()).collect::<Vec<_>>()}
                    }
                }
            }
        }
    }
}
