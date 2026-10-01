//! Watching a shared screen: frames arrive over MoQ, rav1d decodes them on a worker thread, and
//! the pictures are presented on a zgui surface.

use std::sync::Arc;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::Duration;

use zgui::prelude::*;
use zgui::surface::{self as gpu, SurfaceConfig, SurfaceEvent, SurfaceIntrinsic, wgpu};

use crate::engine::Command;
use crate::model::ClientId;
use crate::screen::{Decoder, VideoFrame};
use crate::ui::state::AppState;

/// Decodes `frames` and presents them on `handle` until the surface goes away.
fn play(handle: gpu::SurfaceHandle, frames: Receiver<VideoFrame>) {
    let (events_tx, events_rx) = std::sync::mpsc::channel();
    handle.set_events(move |event| {
        let _ = events_tx.send(event);
    });
    let _ = std::thread::Builder::new()
        .name("screen-view".into())
        .spawn(move || {
            let gpu = loop {
                match events_rx.recv() {
                    Ok(SurfaceEvent::Attached { gpu, .. }) => break gpu,
                    Ok(_) => continue,
                    Err(_) => return,
                }
            };
            let Ok(mut decoder) = Decoder::new() else {
                return;
            };
            let mut ratio = None;
            loop {
                while let Ok(event) = events_rx.try_recv() {
                    if matches!(event, SurfaceEvent::Detached) {
                        return;
                    }
                }
                let first = match frames.recv_timeout(Duration::from_millis(250)) {
                    Ok(f) => f,
                    Err(RecvTimeoutError::Timeout) => continue,
                    Err(RecvTimeoutError::Disconnected) => return,
                };
                // Decode what queued up, a bounded batch at a time; only the newest picture is shown.
                let mut latest = None;
                for frame in std::iter::once(first).chain(frames.try_iter().take(5)) {
                    match decoder.decode(&frame.data) {
                        Ok(Some(picture)) => latest = Some(picture),
                        Ok(None) => {}
                        Err(e) => tracing::debug!("decode: {e:#}"),
                    }
                }
                let Some((w, h, rgba)) = latest else { continue };
                if ratio != Some((w, h)) {
                    handle.set_intrinsic(SurfaceIntrinsic::ratio(w as f32 / h as f32));
                    ratio = Some((w, h));
                }
                let size = wgpu::Extent3d {
                    width: w,
                    height: h,
                    depth_or_array_layers: 1,
                };
                let texture = gpu.device().create_texture(&wgpu::TextureDescriptor {
                    label: Some("screen.frame"),
                    size,
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                    view_formats: &[],
                });
                gpu.queue().write_texture(
                    texture.as_image_copy(),
                    &rgba,
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(w * 4),
                        rows_per_image: None,
                    },
                    size,
                );
                handle.present(Arc::new(texture));
            }
        });
}

/// A live view of `client`'s shared screen. Subscribes while mounted.
#[component]
pub fn ScreenView(state: AppState, client: ClientId, #[prop(into)] class: String) -> impl IntoView {
    let handle = gpu::SurfaceHandle::new(SurfaceConfig::default());
    let (tx, rx) = std::sync::mpsc::channel();
    play(handle.clone(), rx);
    state.send(Command::Watch { client, sink: tx });
    on_cleanup_local(move || state.send(Command::Unwatch { client }));
    view! {
        {zgui::elements::surface().class(class).source(&handle).into_view()}
    }
}

/// Opens a window that shows `client`'s screen large.
pub fn pop_out(state: AppState, client: ClientId, name: String) {
    let windows = use_windows();
    windows.open(
        WindowOptions::new(format!("{name}'s screen — moqspeak"))
            .with_size(1280.0, 800.0)
            .with_stylesheet(POPOUT_SHEET),
        move || {
            view! {
                column(class = "popout") {
                    ScreenView(state = state, client = client, class = "popout-frame")
                }
            }
        },
    );
}

const POPOUT_SHEET: &str = css!(
    ":root { background-color: #000000; }
     .popout { width: 100%; height: 100%; align-items: center; justify-content: center; }
     .popout-frame { width: 100%; height: 100%; }"
);
