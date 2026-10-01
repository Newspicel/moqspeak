//! moqspeak: a TeamSpeak 3 style voice client on zgui and Media over QUIC.

mod audio;
mod bot;
mod engine;
mod media;
mod model;
mod screen;
mod ui;

use zgui::prelude::*;
use zgui_ui_tokens::prelude::*;

use crate::engine::Engine;
use crate::ui::AppState;
use crate::ui::shell::ShellProps;

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
        .with_size(1100.0, 760.0)
        .with_min_size(720.0, 480.0)
        .with_stylesheet(ui::SHEET)
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
            state.info("Welcome to moqspeak. Connect to a server to start talking.");
            // A development aid for screenshots: MOQSPEAK_OPEN=connect|options|about.
            match std::env::var("MOQSPEAK_OPEN").as_deref() {
                Ok("connect") => state.modal.set(ui::state::Modal::Connect),
                Ok("options") => state.modal.set(ui::state::Modal::Options),
                Ok("about") => state.modal.set(ui::state::Modal::About),
                _ => {}
            }
            if let Some(address) = autoconnect.clone() {
                let nickname = autonick
                    .clone()
                    .unwrap_or_else(|| state.settings.with_untracked(|s| s.nickname.clone()));
                state.connect(address, nickname);
            }
            view! {
                ThemeProvider(scheme = Signal::derive_local(move || match state.theme.get() {
                    ui::state::Theme::Dark => ColorScheme::Dark,
                    ui::state::Theme::Light => ColorScheme::Light,
                })) {
                    Shell()
                }
            }
        })
}
