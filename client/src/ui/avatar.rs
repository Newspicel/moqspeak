//! A round avatar with the client's initial, a colour derived from the name, and a ring that
//! lights while the client talks.

use zgui::prelude::*;

const PALETTE: [&str; 10] = [
    "#e5484d", "#f76b15", "#ffc53d", "#46a758", "#12a594", "#0090ff", "#3e63dd", "#8e4ec6",
    "#d6409f", "#978365",
];

/// The colour a name is drawn in, stable across sessions and clients.
pub fn color_of(name: &str) -> &'static str {
    let mut h: u32 = 2166136261;
    for b in name.to_lowercase().bytes() {
        h = (h ^ b as u32).wrapping_mul(16777619);
    }
    PALETTE[(h % PALETTE.len() as u32) as usize]
}

fn initial(name: &str) -> String {
    name.chars()
        .find(|c| c.is_alphanumeric())
        .map(|c| c.to_uppercase().collect())
        .unwrap_or_else(|| "?".into())
}

#[component]
pub fn Avatar(
    #[prop(into)] name: String,
    /// Whether the ring is lit.
    #[prop(into)]
    talking: Signal<bool>,
    #[prop(default = false)] dim: bool,
    #[prop(default = "sm")] size: &'static str,
) -> impl IntoView {
    let color = color_of(&name);
    view! {
        box(
            class = format!("avatar avatar-{size}"),
            class:talking = move || talking.get(),
            class:dim = dim,
            style:background-color = Some(color.to_owned()),
            a11y:hidden = true
        ) {
            text(class = "avatar-initial") {{initial(&name)}}
        }
    }
}
