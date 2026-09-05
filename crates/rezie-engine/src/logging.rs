//! Binary startup logging; never invoked by the clock's steady-state loop.
use std::path::Path;
use tracing_subscriber::prelude::*;

/// Install structured console and nonblocking rolling file logging.
/// Keep the returned guard alive until engine threads are joined.
pub fn init(directory: &Path) -> anyhow::Result<tracing_appender::non_blocking::WorkerGuard> {
    let file = tracing_appender::rolling::Builder::new()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix("rezie")
        .build(directory)?;
    let (writer, guard) = tracing_appender::non_blocking(file);
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .with_writer(writer),
        )
        .try_init()
        .map_err(|error| anyhow::anyhow!("initialize tracing: {error}"))?;
    Ok(guard)
}
