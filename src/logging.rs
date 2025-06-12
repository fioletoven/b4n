use std::path::PathBuf;

use anyhow::Result;
use tracing_error::ErrorLayer;
use tracing_subscriber::{Layer, layer::SubscriberExt, util::SubscriberInitExt};

use crate::core::APP_NAME;

/// Initializes new logging to file and returns worker guard that will flush logs on drop.
pub fn initialize() -> Result<tracing_appender::non_blocking::WorkerGuard> {
    let home_dir = match home::home_dir() {
        Some(mut path) => {
            path.push(format!(".{APP_NAME}/logs"));
            path
        },
        None => PathBuf::from("logs"),
    };
    let appender = tracing_appender::rolling::daily(home_dir, format!("{APP_NAME}.log"));
    let (non_blocking_appender, guard) = tracing_appender::non_blocking(appender);

    let timer = time::format_description::parse("[year]-[month padding:zero]-[day padding:zero] [hour]:[minute]:[second]")?;
    let time_offset = time::UtcOffset::current_local_offset().unwrap_or(time::UtcOffset::UTC);
    let timer = tracing_subscriber::fmt::time::OffsetTime::new(time_offset, timer);

    #[cfg(debug_assertions)]
    let env = format!("warn,{APP_NAME}=info");

    #[cfg(not(debug_assertions))]
    let env = format!("none,{APP_NAME}=info");

    let env_filter = tracing_subscriber::filter::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::filter::EnvFilter::new(env));

    #[cfg(debug_assertions)]
    let file_subscriber = tracing_subscriber::fmt::layer()
        .compact()
        .with_file(true)
        .with_line_number(true)
        .with_target(true)
        .with_thread_ids(true)
        .with_timer(timer)
        .with_ansi(false)
        .with_writer(non_blocking_appender)
        .with_filter(env_filter);

    #[cfg(not(debug_assertions))]
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
