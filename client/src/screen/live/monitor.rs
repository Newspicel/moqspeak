//! A display the user can share.

/// A monitor that can be shared.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct MonitorInfo {
    pub id: u32,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub primary: bool,
}
