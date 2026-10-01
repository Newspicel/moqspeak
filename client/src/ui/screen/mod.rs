//! Screen sharing in the window: the share key, the display picker, the live tag and the view of
//! a shared screen. Without the `screen-share` feature every piece draws nothing.

#[cfg(feature = "screen-share")]
mod live;
#[cfg(not(feature = "screen-share"))]
mod off;

#[cfg(feature = "screen-share")]
pub use crate::ui::screen::live::{LiveTagProps, ShareDialogProps, ShareKeyProps, WatchItemProps};
#[cfg(not(feature = "screen-share"))]
pub use crate::ui::screen::off::{LiveTagProps, ShareDialogProps, ShareKeyProps, WatchItemProps};
