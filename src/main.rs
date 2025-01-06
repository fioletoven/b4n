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
    let client = KubernetesClient::new(args.context(config.context.as_deref()), args.context.is_none()).await?;
    let resource = args.kind(config.get_kind(client.context())).unwrap_or("pods").to_owned();
    let namespace = args.namespace(config.get_namespace(client.context())).map(String::from);

    let mut app = App::new(client, config)?;
    app.start(resource, namespace).await?;

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
