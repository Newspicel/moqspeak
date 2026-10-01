//! The dialogs. One stands open at a time.

use crate::model::{ChannelId, ClientId};

/// Which dialog is open.
#[derive(Clone, Debug, PartialEq)]
pub enum Modal {
    None,
    /// Connect to another server.
    Connect,
    /// Create a channel under `parent`, or at the top.
    CreateChannel {
        parent: Option<ChannelId>,
    },
    /// Poke `to`.
    Poke {
        to: ClientId,
        name: String,
    },
    /// Pick the display to share.
    #[cfg(feature = "screen-share")]
    Share {
        monitors: Vec<crate::screen::MonitorInfo>,
    },
}
