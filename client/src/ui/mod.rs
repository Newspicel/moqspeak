//! The TeamSpeak 3 style interface.

pub mod avatar;
pub mod chat;
pub mod dialogs;
pub mod icons;
pub mod info;
pub mod menu;
pub mod options;
pub mod screen;
pub mod shell;
pub mod state;
pub mod status;
pub mod toolbar;
pub mod tree;

pub use state::AppState;

/// The application style sheet.
pub const SHEET: &str = include_str!("style.css");

/// Erases a view's type so branches of a `match` can return different views.
pub trait IntoAny: zgui::prelude::IntoView + Sized {
    fn into_any(self) -> zgui::prelude::AnyView {
        zgui::prelude::AnyView::new(self)
    }
}

impl<T: zgui::prelude::IntoView> IntoAny for T {}
