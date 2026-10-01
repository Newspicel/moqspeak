//! The window frame: the layout of the regions, who draws the frame, how the window stands on the
//! screen, the window keys, the resize edges and the drag label.

mod controls;
mod edges;
mod ghost;
mod placement;
mod source;
mod view;

pub use crate::ui::frame::view::FrameProps;
