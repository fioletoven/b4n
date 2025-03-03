use std::path::PathBuf;

use anyhow::Result;
use tracing_error::ErrorLayer;
use tracing_subscriber::{Layer, layer::SubscriberExt, util::SubscriberInitExt};

/// Initializes new logging to file and returns worker guard that will flush logs on drop.
pub fn initialize() -> Result<tracing_appender::non_blocking::WorkerGuard> {
    let home_dir = match home::home_dir() {
        Some(mut path) => {
            path.push(format!(".{}/logs", env!("CARGO_CRATE_NAME")));
            path
        }
        None => PathBuf::from("logs"),
    };
    let appender = tracing_appender::rolling::daily(home_dir, format!("{}.log", env!("CARGO_CRATE_NAME")));
    let (non_blocking_appender, guard) = tracing_appender::non_blocking(appender);

    let timer = time::format_description::parse("[year]-[month padding:zero]-[day padding:zero] [hour]:[minute]:[second]")?;
    let time_offset = time::UtcOffset::current_local_offset().unwrap_or(time::UtcOffset::UTC);
    let timer = tracing_subscriber::fmt::time::OffsetTime::new(time_offset, timer);

    let env_filter = tracing_subscriber::filter::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::filter::EnvFilter::new(format!("none,{}=info", env!("CARGO_CRATE_NAME"))));

    let file_subscriber = tracing_subscriber::fmt::layer()
        .compact()
        .with_target(true)
        .with_thread_ids(true)
        .with_timer(timer)
        .with_ansi(false)
        .with_writer(non_blocking_appender)
        .with_filter(env_filter);

    tracing_subscriber::registry()
        .with(file_subscriber)
        .with(ErrorLayer::default())
        .init();

    Ok(guard)
}
