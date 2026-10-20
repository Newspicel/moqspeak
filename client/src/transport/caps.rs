//! What the live transport can do. The interface offers a control only where the transport behind
//! it answers to that control, so nothing presents a button that cannot work.

/// The optional features a transport carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Capabilities {
    /// Offers and watches shared screens.
    pub screen_share: bool,
    /// Creates, edits and deletes channels.
    pub channels: bool,
    /// Assigns roles.
    pub roles: bool,
    /// Removes a client from the server.
    pub kick: bool,
    /// Reports the mute, deafen and away state of other clients as it changes.
    pub live_status: bool,
}

impl Capabilities {
    /// Offline, where the interface offers nothing.
    pub const NONE: Self = Self {
        screen_share: false,
        channels: false,
        roles: false,
        kick: false,
        live_status: false,
    };

    /// Everything, which is what the moqspeak Worker serves.
    pub const MOQ: Self = Self {
        screen_share: true,
        channels: true,
        roles: true,
        kick: true,
        live_status: true,
    };

    /// What a TeamSpeak 3 server answers to. It has server groups in place of roles, carries no
    /// screens, and names its channel rights per group, so the client leaves channel editing to
    /// the official client.
    pub const TS3: Self = Self {
        screen_share: false,
        channels: false,
        roles: false,
        kick: true,
        live_status: true,
    };
}

impl Default for Capabilities {
    fn default() -> Self {
        Self::NONE
    }
}
