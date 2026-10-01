//! The strips along the window's edges that resize it.

use zgui::prelude::*;

use crate::ui::frame::controls::own_frame;
use crate::ui::parts::Erase;

/// Every edge and corner, with the class that places it.
const EDGES: [(ResizeEdge, &str); 8] = [
    (ResizeEdge::North, "ms-edge ms-edge--n"),
    (ResizeEdge::South, "ms-edge ms-edge--s"),
    (ResizeEdge::West, "ms-edge ms-edge--w"),
    (ResizeEdge::East, "ms-edge ms-edge--e"),
    (ResizeEdge::NorthWest, "ms-edge ms-edge--nw"),
    (ResizeEdge::NorthEast, "ms-edge ms-edge--ne"),
    (ResizeEdge::SouthWest, "ms-edge ms-edge--sw"),
    (ResizeEdge::SouthEast, "ms-edge ms-edge--se"),
];

/// The grab strips, where the window draws its own frame.
#[component]
pub fn ResizeEdges() -> impl IntoView {
    let Some(window) = try_use_window() else {
        return ().any();
    };
    let strips = EDGES
        .into_iter()
        .map(|(edge, class)| {
            view! { box(class = class, on:pointer_down = window.resize_drag_handler(edge)) }.any()
        })
        .collect::<Vec<_>>();
    own_frame(view! { box(class = "ms-edges") {{strips}} })
}
