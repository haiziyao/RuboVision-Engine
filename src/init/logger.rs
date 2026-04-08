use anyhow::Result;
use tracing_appender::rolling;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

pub fn init_logging() -> Result<tracing_appender::non_blocking::WorkerGuard> {
    std::fs::create_dir_all("logs")?;

    let file_appender = rolling::daily("logs", "rubovision.log");
    let (file_writer, guard) = tracing_appender::non_blocking(file_appender);

    let stdout_layer = fmt::layer()
        .with_ansi(true)
        .with_target(false)
        .with_line_number(false);

    let file_layer = fmt::layer()
        .with_ansi(false)
        .with_writer(file_writer)
        .with_target(true)
        .with_line_number(true)
        .with_file(true);

    tracing_subscriber::registry()
        .with(EnvFilter::new("info"))
        .with(stdout_layer)
        .with(file_layer)
        .init();

    Ok(guard)
}