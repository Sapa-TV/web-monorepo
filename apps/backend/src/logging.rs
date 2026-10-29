use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling;
use tracing_subscriber::layer::Layer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, fmt};

pub fn init() -> Option<WorkerGuard> {
    let console_filter = EnvFilter::try_from_default_env().unwrap_or(EnvFilter::new("info"));
    let file_filter = EnvFilter::try_from_env("RUST_LOG_FILE").unwrap_or(EnvFilter::new("debug"));

    let file_logging = rolling::Builder::new()
        .rotation(rolling::Rotation::DAILY)
        .filename_prefix("backend.log")
        .build("logs")
        .map(tracing_appender::non_blocking)
        .map(|(writer, guard)| {
            let layer = fmt::layer()
                .with_writer(writer)
                .with_ansi(false)
                .with_filter(file_filter);
            (layer, guard)
        })
        .map_err(|e| eprintln!("file logging disabled: {e}"))
        .ok();
    let (file_layer, guard) = match file_logging {
        Some((layer, guard)) => (Some(layer), Some(guard)),
        None => (None, None),
    };

    tracing_subscriber::registry()
        .with(fmt::layer().with_filter(console_filter))
        .with(file_layer)
        .init();
    guard
}
