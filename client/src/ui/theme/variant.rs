//! The themes the interface offers.
//!
//! A theme is a block of custom-property declarations laid over the base token set. Each one is
//! two files under `assets/themes`, one for a light surface and one for a dark surface, compiled
//! in. Adding a theme is two files and one line in the table below.

use zgui_ui_tokens::prelude::*;

use crate::ui::theme::swatch::{Swatch, swatch_of};

/// Declares the themes and reads their two files.
macro_rules! themes {
    ($($name:ident => ($file:literal, $label:literal),)*) => {
        /// A theme the interface offers.
        #[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
        pub enum Variant {
            $(
                #[doc = concat!("The ", $label, " theme.")]
                $name,
            )*
        }

        impl Variant {
            /// Every theme, in the order they are offered.
            pub const ALL: &'static [Self] = &[$(Self::$name),*];

            /// How this is written in the settings file.
            pub const fn name(self) -> &'static str {
                match self {
                    $(Self::$name => $file,)*
                }
            }

            /// What it is called in the interface.
            pub const fn label(self) -> &'static str {
                match self {
                    $(Self::$name => $label,)*
                }
            }

            /// The declarations it lays over the light token set.
            pub const fn light_css(self) -> &'static str {
                match self {
                    $(
                        Self::$name => include_str!(
                            concat!("../../../assets/themes/", $file, "-light.css")
                        ),
                    )*
                }
            }

            /// The declarations it lays over the dark token set.
            pub const fn dark_css(self) -> &'static str {
                match self {
                    $(
                        Self::$name => include_str!(
                            concat!("../../../assets/themes/", $file, "-dark.css")
                        ),
                    )*
                }
            }
        }
    };
}

themes! {
    Jellybeans => ("jellybeans", "Jellybeans"),
    Ayu => ("ayu", "Ayu"),
    Catppuccin => ("catppuccin", "Catppuccin"),
    Dracula => ("dracula", "Dracula"),
    Edge => ("edge", "Edge"),
    Everforest => ("everforest", "Everforest"),
    Flexoki => ("flexoki", "Flexoki"),
    Github => ("github", "GitHub"),
    Gruvbox => ("gruvbox", "Gruvbox"),
    GruvboxMaterial => ("gruvbox-material", "Gruvbox Material"),
    Iceberg => ("iceberg", "Iceberg"),
    Kanagawa => ("kanagawa", "Kanagawa"),
    Material => ("material", "Material"),
    Melange => ("melange", "Melange"),
    Modus => ("modus", "Modus"),
    Monokai => ("monokai", "Monokai Pro"),
    NightOwl => ("night-owl", "Night Owl"),
    Nightfox => ("nightfox", "Nightfox"),
    Nord => ("nord", "Nord"),
    One => ("one", "One"),
    Oxocarbon => ("oxocarbon", "Oxocarbon"),
    Papercolor => ("papercolor", "PaperColor"),
    RosePine => ("rose-pine", "Rose Pine"),
    Solarized => ("solarized", "Solarized"),
    TokyoNight => ("tokyo-night", "Tokyo Night"),
    Vitesse => ("vitesse", "Vitesse"),
}

impl Default for Variant {
    /// Jellybeans, the theme a new installation wears.
    fn default() -> Self {
        Self::Jellybeans
    }
}

impl Variant {
    /// The theme written as `name`, if there is one.
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|it| it.name() == name)
    }

    /// This theme as the one for a light surface.
    pub fn light(self) -> Theme {
        Theme::light().with_css(self.light_css())
    }

    /// This theme as the one for a dark surface.
    pub fn dark(self) -> Theme {
        Theme::dark().with_css(self.dark_css())
    }

    /// The colours a preview of this theme draws at the dark or the light surface.
    pub fn swatch(self, dark: bool) -> Swatch {
        swatch_of(if dark {
            self.dark_css()
        } else {
            self.light_css()
        })
    }
}
