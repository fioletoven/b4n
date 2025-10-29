use anyhow::Result;
use b4n_kube::PODS;
use b4n_kube::client::get_context;
use clap::Parser;
use core::{App, Config, ExecutionFlow, History};
use std::thread::sleep;
use std::time::{Duration, Instant};
use tokio::runtime::Builder;
use tracing::{error, info};

pub mod cli;
pub mod core;
pub mod kubernetes;
pub mod ui;

fn main() -> Result<()> {
    let args = cli::Args::parse();

    let _logging_guard = b4n_utils::logging::initialize(core::APP_NAME)?;
    info!("{} v{} started", core::APP_NAME, core::APP_VERSION);

    if let Err(error) = run_application(&args) {
        error!(
            "{} v{} terminated with an error: {}",
            core::APP_NAME,
            core::APP_VERSION,
            error
        );
        Err(error)
    } else {
        info!("{} v{} stopped", core::APP_NAME, core::APP_VERSION);
        Ok(())
    }
}

fn run_application(args: &cli::Args) -> Result<()> {
    let rt = Builder::new_multi_thread().enable_all().build()?;

    let mut history = rt.block_on(History::load_or_create())?;
    let (context, kube_config_path) = rt.block_on(get_context(
        args.kube_config.as_deref(),
        args.context(history.current_context()),
        args.context.is_none(),
    ))?;
    let Some(context) = context else {
        return Err(anyhow::anyhow!(format!(
            "Kube context '{}' not found in configuration.",
            args.context(history.current_context()).unwrap_or("default")
        )));
    };
    history.set_kube_config_path(kube_config_path);

    let kind = args.kind(history.get_kind(&context)).unwrap_or(PODS).into();
    let namespace = args.namespace(history.get_namespace(&context)).map(String::from).into();

    let config = rt.block_on(Config::load_or_create())?;
    let theme = rt.block_on(config.load_or_create_theme())?;
    let mut app = App::new(rt.handle().clone(), config, history, theme, args.insecure)?;

    app.start(context, kind, namespace)?;
    application_loop(&mut app)?;
    app.stop()?;

    Ok(())
}

fn application_loop(app: &mut App) -> Result<(), anyhow::Error> {
    const FPS: u64 = 20;
    const FRAME_DURATION: Duration = Duration::from_nanos(1_000_000_000 / FPS);

    loop {
        let frame_start = Instant::now();
        if app.process_events()? == ExecutionFlow::Stop {
            break;
        }

        app.draw_frame()?;

        let frame_time = frame_start.elapsed();
        if frame_time < FRAME_DURATION {
            sleep(FRAME_DURATION - frame_time);
        }
    }

    Ok(())
}
