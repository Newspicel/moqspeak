//! The small pieces every region uses: icons, status dots, tags, ghost keys and press handlers.

mod dot;
mod erase;
pub mod icons;
mod key;
pub mod press;
mod tag;

pub use crate::ui::parts::dot::StatusDotProps;
pub use crate::ui::parts::erase::Erase;
pub use crate::ui::parts::key::KeyProps;
pub use crate::ui::parts::tag::TagProps;

/// A menu callback from a plain closure.
pub fn act(f: impl Fn() + 'static) -> Option<zgui::reactive::UnsyncCallback<()>> {
    Some(zgui::reactive::UnsyncCallback::new(move |_: ()| f()))
}
