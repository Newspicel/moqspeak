//! The few colours a theme shows itself by.
//!
//! The theme picker draws a preview of each theme. The colours come out of the theme's own file,
//! so the picker shows what the window will look like and no Rust holds a second copy of a
//! palette.

/// The colours a theme preview draws.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Swatch {
    /// The page behind everything.
    pub background: &'static str,
    /// The plane beside the page.
    pub sidebar: &'static str,
    /// The tone of the main action.
    pub primary: &'static str,
    /// Ordinary text.
    pub foreground: &'static str,
}

/// The properties a swatch reads, in the order the fields sit in.
pub const NAMES: [&str; 4] = [
    "--zui-color-background",
    "--zui-color-sidebar",
    "--zui-color-primary",
    "--zui-color-foreground",
];

/// What a swatch shows for a property the file leaves out.
const ABSENT: &str = "transparent";

/// The swatch `css` declares.
pub fn swatch_of(css: &'static str) -> Swatch {
    Swatch {
        background: declared(css, NAMES[0]).unwrap_or(ABSENT),
        sidebar: declared(css, NAMES[1]).unwrap_or(ABSENT),
        primary: declared(css, NAMES[2]).unwrap_or(ABSENT),
        foreground: declared(css, NAMES[3]).unwrap_or(ABSENT),
    }
}

/// The value `css` declares `property` with. The last declaration wins, as in the cascade.
pub fn declared(css: &'static str, property: &str) -> Option<&'static str> {
    css.lines()
        .filter_map(|line| {
            let rest = line.trim_start().strip_prefix(property)?;
            let value = rest.strip_prefix(':')?.trim();
            Some(value.strip_suffix(';').unwrap_or(value).trim())
        })
        .rfind(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::{NAMES, declared};
    use crate::ui::theme::Variant;

    #[test]
    fn a_longer_name_is_not_read_as_the_shorter_one_it_starts_with() {
        let css = "--zui-color-primary-foreground: a;\n--zui-color-primary: b;\n";
        assert_eq!(declared(css, "--zui-color-primary"), Some("b"));
    }

    #[test]
    fn every_theme_shows_a_whole_swatch_at_both_surfaces() {
        for variant in Variant::ALL {
            for css in [variant.light_css(), variant.dark_css()] {
                for name in NAMES {
                    assert!(
                        declared(css, name).is_some(),
                        "{} declares no {name}",
                        variant.name()
                    );
                }
            }
        }
    }
}
