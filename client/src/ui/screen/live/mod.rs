//! Screen sharing in the window as the `screen-share` feature builds it.

mod live_tag;
mod picker;
mod share_dialog;
mod share_key;
mod toggle;
mod viewer;
mod watch_item;

pub use crate::ui::screen::live::live_tag::LiveTagProps;
pub use crate::ui::screen::live::share_dialog::ShareDialogProps;
pub use crate::ui::screen::live::share_key::ShareKeyProps;
pub use crate::ui::screen::live::watch_item::WatchItemProps;
