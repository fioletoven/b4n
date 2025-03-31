use anyhow::Result;
use app::{App, Config, ExecutionFlow, History};
use clap::Parser;
use kubernetes::{client::get_context, resources::PODS};
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
    let args = cli::Args::parse();

    let _logging_guard = logging::initialize()?;
    info!("{} v{} started", app::APP_NAME, app::APP_VERSION);

    if let Err(error) = run_application(args).await {
        error!("{} v{} terminated with an error: {}", app::APP_NAME, app::APP_VERSION, error);
        Err(error)
    } else {
        info!("{} v{} stopped", app::APP_NAME, app::APP_VERSION);
        Ok(())
    }
}

async fn run_application(args: cli::Args) -> Result<()> {
    let mut history = History::load_or_create().await?;
    let (context, kube_config_path) = get_context(
        args.kube_config.as_deref(),
        args.context(history.current_context()),
        args.context.is_none(),
    )
    .await?;
    let Some(context) = context else {
        return Err(anyhow::anyhow!(format!(
            "Kube context '{}' not found in configuration.",
            args.context(history.current_context()).unwrap_or("default")
        )));
    };
    history.set_kube_config_path(kube_config_path);

    let resource = args.kind(history.get_kind(&context)).unwrap_or(PODS).to_owned();
    let namespace = args.namespace(history.get_namespace(&context)).map(String::from);

    let config = Config::load_or_create().await?;
    let theme = config.load_or_create_theme().await?;
    let mut app = App::new(config, history, theme)?;
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
