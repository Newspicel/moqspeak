//! The desktop backend the app runs on. zgui creates a window hidden and shows it after its first
//! frame presents. Windows paints no hidden window, so a frame never starts there. On Windows this
//! backend shows each window as soon as it exists.

use std::collections::HashSet;

use zgui::platform::{
    AppHandler, IdlePolicy, PlatformCx, PlatformError, SurfaceEvent, SurfaceId, WakeReason,
};

/// Runs `handler` on the desktop backend.
pub fn run(handler: Box<dyn AppHandler>) -> Result<(), PlatformError> {
    let driver = zgui::app::desktop();
    if cfg!(windows) {
        driver(Box::new(ShowOnCreate {
            inner: handler,
            shown: HashSet::new(),
        }))
    } else {
        driver(handler)
    }
}

/// Forwards every callback and shows each window that appeared during it.
struct ShowOnCreate {
    inner: Box<dyn AppHandler>,
    /// The windows already shown. Surface ids are never reused.
    shown: HashSet<SurfaceId>,
}

impl ShowOnCreate {
    /// Shows the windows created since the last call.
    fn show_new(&mut self, cx: &dyn PlatformCx) {
        for surface in cx.surfaces() {
            if self.shown.insert(surface.id()) {
                surface.set_visible(true);
            }
        }
    }
}

impl AppHandler for ShowOnCreate {
    fn surfaces_available(&mut self, cx: &dyn PlatformCx) {
        self.inner.surfaces_available(cx);
        self.show_new(cx);
    }

    fn surfaces_lost(&mut self, cx: &dyn PlatformCx) {
        self.inner.surfaces_lost(cx);
    }

    fn surface_event(&mut self, cx: &dyn PlatformCx, surface: SurfaceId, event: SurfaceEvent) {
        self.inner.surface_event(cx, surface, event);
        self.show_new(cx);
    }

    fn wake(&mut self, cx: &dyn PlatformCx, reason: WakeReason) {
        self.inner.wake(cx, reason);
        self.show_new(cx);
    }

    fn idle(&mut self, cx: &dyn PlatformCx) -> IdlePolicy {
        self.inner.idle(cx)
    }

    fn deadline_reached(&mut self, cx: &dyn PlatformCx) {
        self.inner.deadline_reached(cx);
        self.show_new(cx);
    }

    fn shutting_down(&mut self, cx: &dyn PlatformCx) {
        self.inner.shutting_down(cx);
    }
}
