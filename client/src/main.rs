//! moqspeak: a voice client on zgui and Media over QUIC.

// A release build on Windows is a GUI program with no console window.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod audio;
mod bot;
mod engine;
mod identity;
mod media;
mod model;
mod screen;
mod ui;

use zgui::prelude::*;

use crate::engine::Engine;
use crate::ui::AppState;
use crate::ui::RootProps;

/// The window icon, decoded from the PNG the build embeds.
fn window_icon() -> WindowIcon {
    let png = include_bytes!("../../packaging/icon/moqspeak-256.png");
    let image = image::load_from_memory_with_format(png, image::ImageFormat::Png)
        .expect("embedded icon is a valid PNG")
        .into_rgba8();
    let (w, h) = image.dimensions();
    WindowIcon::from_rgba(image.into_raw(), w, h).expect("icon size matches its pixels")
}

fn main() -> Result<(), zgui::Error> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "warn,moqspeak=info,moq_tokio=info".into()),
        )
        .init();

    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("--bot") {
        bot::run(&args[2..]);
        return Ok(());
    }

    // `moqspeak <address> [nickname]` connects at start-up.
    let autoconnect = args.get(1).filter(|a| !a.starts_with('-')).cloned();
    let autonick = args.get(2).cloned();
    let saved = ui::state::Settings::load();
    let mut boot = Some(Engine::start(
        saved.input_device.clone(),
        saved.output_device.clone(),
    ));

    app()
        .with_application_id("dev.moqspeak.Client")
        .with_title("moqspeak")
        .with_size(1100.0, 720.0)
        .with_min_size(640.0, 440.0)
        .with_decorations(Decorations::NoTitleBar)
        .with_icon(window_icon())
        // Pop-out screen windows have no life of their own.
        .with_exit_policy(ExitPolicy::WhenPrimaryCloses)
        .with_stylesheet(ui::sheet())
        .run(move || {
            let (engine, mut events) = boot.take().expect("one main window");
            let state = AppState::new(engine);
            provide_local_context(state);
            // Network events land on the UI thread through the channel.
            let _pump = spawn_local(async move {
                while let Some(event) = events.recv().await {
                    state.apply(event);
                }
            });
            // Closing the main window ends the program at once. Settings are saved as they change
            // and the OS closes the sockets; tearing the tree down piece by piece only gives
            // late reactive work a chance to touch values that are already gone.
            std::mem::forget(on_close_request(|| std::process::exit(0)));
            // A development aid for screenshots: MOQSPEAK_OPEN=connect,settings,appearance,log,chat.
            for open in std::env::var("MOQSPEAK_OPEN")
                .unwrap_or_default()
                .split(',')
            {
                match open {
                    "connect" => state.modal.set(ui::state::Modal::Connect),
                    "settings" => state.settings_open.set(true),
                    "appearance" => {
                        state.settings_page.set("appearance".into());
                        state.settings_open.set(true);
                    }
                    "log" => state.log_open.set(true),
                    "chat" => state.chat.update(|c| c.toggle()),
                    _ => {}
                }
            }
            if let Some(address) = autoconnect.clone() {
                let nickname = autonick
                    .clone()
                    .unwrap_or_else(|| state.settings.with_untracked(|s| s.nickname.clone()));
                state.connect(address, nickname);
            }
            view! { Root() }
        })
}
