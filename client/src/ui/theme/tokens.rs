//! The custom properties a theme carries that the token schema has no field for.
//!
//! `ThemeProvider` writes out the `--zui-*` tokens it knows. A theme file also carries the status
//! tones and the menu hover. This component writes the whole file out as a sheet of its own,
//! under the same scheme rules the provider uses.

use zgui::prelude::*;
use zgui::reactive::{LocalStorage, RenderEffect};
use zgui_ui_tokens::prelude::*;

use crate::ui::theme::Variant;

/// What the sheet is installed under.
const SHEET: &str = "ms-tokens";

/// The declarations `variant` carries at `scheme`.
fn sheet(variant: Variant, scheme: ColorScheme) -> String {
    let mut css = String::new();
    if scheme.wants_light() {
        css.push_str(":root {\n");
        css.push_str(variant.light_css());
        css.push_str("\n}\n");
    }
    if scheme.wants_dark() {
        let rule = format!(":root {{\n{}\n}}\n", variant.dark_css());
        // Pinned to dark, the properties are unconditional. Left to the desktop, they are the
        // override the media query switches on.
        if scheme == ColorScheme::System {
            css.push_str("@media (prefers-color-scheme: dark) {\n");
            css.push_str(&rule);
            css.push_str("}\n");
        } else {
            css.push_str(&rule);
        }
    }
    css
}

/// Puts the application's own custom properties into the window, and keeps them there.
#[component]
pub fn MsTokens(
    /// Which theme the properties come from.
    #[prop(into)]
    variant: Signal<Variant, LocalStorage>,
    /// Which scheme they are written for.
    #[prop(into)]
    scheme: Signal<ColorScheme, LocalStorage>,
) -> impl IntoView {
    let installed = Stylesheet::install(
        SHEET,
        &sheet(variant.get_untracked(), scheme.get_untracked()),
    );

    let following = RenderEffect::new(move |previous: Option<()>| {
        let css = sheet(variant.get(), scheme.get());
        if previous.is_some()
            && let Some(installed) = installed.as_ref()
        {
            installed.replace(&css);
        }
    });
    on_cleanup_local(move || drop(following));

    view! { box(style:display = "none") }
}
