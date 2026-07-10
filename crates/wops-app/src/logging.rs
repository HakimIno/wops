use std::{fs, io};

use anyhow::Context;
use tracing_appender::{non_blocking::WorkerGuard, rolling};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};
use wops_core::Settings;

pub fn init() -> anyhow::Result<WorkerGuard> {
    let log_directory = Settings::data_dir()
        .context("could not determine application data directory")?
        .join("logs");
    fs::create_dir_all(&log_directory)
        .with_context(|| format!("could not create log directory {}", log_directory.display()))?;

    let file_appender = rolling::daily(log_directory, "wops.log");
    let (file_writer, guard) = tracing_appender::non_blocking(file_appender);
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("wops=info,wops_app=info,wops_core=info"));

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().with_writer(io::stdout))
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .with_writer(file_writer),
        )
        .try_init()
        .context("could not install tracing subscriber")?;

    Ok(guard)
}

pub fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        tracing::error!(%panic_info, "application panicked");
        default_hook(panic_info);
    }));
}
