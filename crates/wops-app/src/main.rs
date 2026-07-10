mod app;
mod logging;

use anyhow::Context;
use eframe::egui;
use tracing::info;
use wops_core::Settings;

fn main() -> anyhow::Result<()> {
    let _log_guard = logging::init().context("failed to initialize logging")?;
    logging::install_panic_hook();
    info!(version = env!("CARGO_PKG_VERSION"), "starting WOPS");

    let settings = Settings::load().unwrap_or_else(|error| {
        tracing::error!(%error, "could not load settings; using defaults");
        Settings::default()
    });
    let viewport = egui::ViewportBuilder::default()
        .with_app_id("wops")
        .with_title("WOPS")
        .with_inner_size([settings.window_width, settings.window_height])
        .with_min_inner_size([900.0, 600.0]);
    let native_options = eframe::NativeOptions {
        viewport,
        renderer: eframe::Renderer::Wgpu,
        persist_window: true,
        ..Default::default()
    };

    eframe::run_native(
        "WOPS",
        native_options,
        Box::new(move |creation_context| {
            Ok(Box::new(app::WopsApp::new(creation_context, settings)))
        }),
    )
    .map_err(|error| anyhow::anyhow!(error.to_string()))
}
