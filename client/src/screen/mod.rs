//! Screen sharing. The `screen-share` feature builds it. Without the feature, an idle share
//! stands in.

#[cfg(feature = "screen-share")]
mod live;
#[cfg(not(feature = "screen-share"))]
mod off;

#[cfg(feature = "screen-share")]
pub use crate::screen::live::*;
#[cfg(not(feature = "screen-share"))]
pub use crate::screen::off::Share;
