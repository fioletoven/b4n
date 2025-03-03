use anyhow::Result;
use app::{App, Config, ExecutionFlow};
use clap::Parser;
use kubernetes::client::get_context;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, info};

pub mod app;
pub mod cli;
pub mod kubernetes;
pub mod logging;
pub mod ui;
pub mod utils;

#[tokio::main]
async fn main() -> Result<()> {
    let _logging_guard = logging::initialize()?;
    info!("{} v{} started", env!("CARGO_CRATE_NAME"), env!("CARGO_PKG_VERSION"));

    if let Err(error) = run_application().await {
        error!(
            "{} v{} terminated with an error: {}",
            env!("CARGO_CRATE_NAME"),
            env!("CARGO_PKG_VERSION"),
            error
        );
        Err(error)
    } else {
        info!("{} v{} stopped", env!("CARGO_CRATE_NAME"), env!("CARGO_PKG_VERSION"));
        Ok(())
    }
}

async fn run_application() -> Result<()> {
    let args = cli::Args::parse();

    let mut config = Config::load_or_create().await?;
    let (context, kube_config_path) = get_context(
        args.kube_config.as_deref(),
        args.context(config.current_context()),
        args.context.is_none(),
    )
    .await?;
    let Some(context) = context else {
        return Err(anyhow::anyhow!(format!(
            "Kube context '{}' not found in configuration.",
            args.context(config.current_context()).unwrap_or("default")
        )));
    };
    config.set_kube_config_path(kube_config_path);

    let resource = args.kind(config.get_kind(&context)).unwrap_or("pods").to_owned();
    let namespace = args.namespace(config.get_namespace(&context)).map(String::from);

    let mut app = App::new(config)?;
    app.start(context, resource, namespace.into()).await?;

    loop {
        if app.process_events()? == ExecutionFlow::Stop {
            break;
        }

        app.draw_frame()?;

        sleep(Duration::from_millis(50)).await;
    }

    app.stop()?;
    Ok(())
}
