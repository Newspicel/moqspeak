//! Making two views one type.

/// Erases a view, so two branches of a choice can return different views.
pub trait Erase {
    /// This view, with its type forgotten.
    fn any(self) -> zgui::view::AnyView;
}

impl<V: zgui::prelude::IntoView> Erase for V {
    fn any(self) -> zgui::view::AnyView {
        zgui::view::AnyView::new(self)
    }
}
