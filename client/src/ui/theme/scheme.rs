//! Which surface the interface presents on.

use zgui_ui_tokens::prelude::*;

/// The surface: follow the desktop, or hold one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scheme {
    System,
    Dark,
    Light,
}

impl Scheme {
    /// Every surface, in the order they are offered.
    pub const ALL: [Self; 3] = [Self::System, Self::Dark, Self::Light];

    pub fn parse(s: &str) -> Self {
        match s {
            "light" => Self::Light,
            "system" => Self::System,
            _ => Self::Dark,
        }
    }

    /// How this is written in the settings file.
    pub fn name(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Dark => "dark",
            Self::Light => "light",
        }
    }

    /// What it is called in the interface.
    pub fn label(self) -> &'static str {
        match self {
            Self::System => "System",
            Self::Dark => "Dark",
            Self::Light => "Light",
        }
    }

    /// The scheme as the token library writes it.
    pub fn color_scheme(self) -> ColorScheme {
        match self {
            Self::System => ColorScheme::System,
            Self::Dark => ColorScheme::Dark,
            Self::Light => ColorScheme::Light,
        }
    }
}
