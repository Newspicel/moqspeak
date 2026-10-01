//! The property set every theme file declares.
//!
//! A theme that leaves a property out reads the base token for it, which shows as a colour from
//! another theme. The tests hold each file to the set Jellybeans declares.

/// The property names `css` declares, in the order it declares them.
fn names(css: &str) -> Vec<&str> {
    css.lines()
        .map(str::trim)
        .filter_map(|line| line.strip_prefix("--")?.split_once(':'))
        .map(|(name, _)| name)
        .collect()
}

/// The value `css` declares for the property `name`.
fn value<'a>(css: &'a str, name: &str) -> Option<&'a str> {
    css.lines()
        .map(str::trim)
        .filter_map(|line| line.strip_prefix("--")?.split_once(':'))
        .find(|(declared, _)| *declared == name)
        .map(|(_, value)| value.trim().trim_end_matches(';'))
}

/// The lightness of an `oklch(...)` colour, from zero to one.
fn lightness(colour: &str) -> Option<f32> {
    colour
        .strip_prefix("oklch(")?
        .split_whitespace()
        .next()?
        .parse()
        .ok()
}

#[cfg(test)]
mod tests {
    use super::{lightness, names, value};
    use crate::ui::theme::Variant;

    #[test]
    fn every_theme_declares_the_whole_set_at_both_surfaces() {
        let whole = names(Variant::Jellybeans.dark_css());
        for variant in Variant::ALL {
            for (surface, css) in [("light", variant.light_css()), ("dark", variant.dark_css())] {
                let declared = names(css);
                for name in &whole {
                    assert!(
                        declared.contains(name),
                        "{} {surface} declares no --{name}",
                        variant.name()
                    );
                }
                assert_eq!(
                    declared.len(),
                    whole.len(),
                    "{} {surface} declares a property twice or one Jellybeans leaves out",
                    variant.name()
                );
            }
        }
    }

    #[test]
    fn a_theme_holds_only_library_and_application_tokens() {
        for variant in Variant::ALL {
            for css in [variant.light_css(), variant.dark_css()] {
                for name in names(css) {
                    assert!(
                        name.starts_with("zui-") || name.starts_with("ms-"),
                        "{} declares --{name}",
                        variant.name()
                    );
                }
            }
        }
    }

    #[test]
    fn every_menu_lights_the_row_under_the_pointer_one_step_off_its_surface() {
        for variant in Variant::ALL {
            for (surface, css) in [("light", variant.light_css()), ("dark", variant.dark_css())] {
                let read = |name: &str| {
                    value(css, name)
                        .and_then(lightness)
                        .unwrap_or_else(|| panic!("{} {surface} writes --{name}", variant.name()))
                };
                let popover = read("zui-color-popover");
                let text = read("zui-color-popover-foreground");
                let hover = read("ms-menu-hover");
                assert!(
                    (hover - popover).abs() >= 0.06,
                    "{} {surface} lights a menu row too faintly",
                    variant.name()
                );
                assert!(
                    (hover - popover).signum() == (text - popover).signum(),
                    "{} {surface} lights a menu row away from its text",
                    variant.name()
                );
            }
        }
    }
}
