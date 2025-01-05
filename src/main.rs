use anyhow::Result;
use app::{App, Config, ExecutionFlow};
use clap::Parser;
use kubernetes::client::KubernetesClient;
use std::time::Duration;
use tokio::time::sleep;
use tracing::info;

pub mod app;
pub mod cli;
pub mod kubernetes;
pub mod logging;
pub mod ui;
pub mod utils;

#[tokio::main]
async fn main() -> Result<()> {
    let _logging_guard = logging::utils::initialize()?;
    info!("{} v{} started", env!("CARGO_CRATE_NAME"), env!("CARGO_PKG_VERSION"));

    let args = cli::Args::parse();
    let config = Config::load_or_create().await?;
    let client = KubernetesClient::new(args.context.as_deref()).await?;

    let mut app = App::new(client, config)?;
    app.start(args.resource.clone(), args.namespace()).await?;

    loop {
        sleep(Duration::from_millis(50)).await;

        app.draw_frame()?;
        if app.process_events()? == ExecutionFlow::Stop {
            break;
        }
    }

    app.stop()?;

    info!("{} v{} stopped", env!("CARGO_CRATE_NAME"), env!("CARGO_PKG_VERSION"));
    Ok(())
}
