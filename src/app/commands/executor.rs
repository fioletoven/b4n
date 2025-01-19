use tokio::{
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;

use crate::app::utils::wait_for_task;

use super::{ExecutorCommand, ExecutorResult};

/// Background commands executor.
#[derive(Default)]
pub struct BgExecutor {
    task: Option<JoinHandle<()>>,
    cancellation_token: Option<CancellationToken>,
    commands_tx: Option<UnboundedSender<ExecutorCommand>>,
    results_rx: Option<UnboundedReceiver<ExecutorResult>>,
}

impl BgExecutor {
    /// Starts background task for commands execution.
    pub fn start(&mut self) {
        if self.cancellation_token.is_some() {
            return;
        }

        let cancellation_token = CancellationToken::new();
        let _cancellation_token = cancellation_token.clone();

        let (commands_tx, mut _commands_rx) = mpsc::unbounded_channel();
        let (_results_tx, results_rx) = mpsc::unbounded_channel();

        let task = tokio::spawn(async move {
            while !_cancellation_token.is_cancelled() {
                let command = tokio::select! {
                    _ = _cancellation_token.cancelled() => break,
                    v = _commands_rx.recv() => v,
                };

                let Some(command) = command else {
                    break;
                };

                let result = match command {
                    ExecutorCommand::ListKubeContexts(command) => command.execute().await,
                    ExecutorCommand::NewKubernetesClient(command) => command.execute().await,
                    ExecutorCommand::SaveConfiguration(command) => command.execute().await,
                    ExecutorCommand::DeleteResource(mut command) => command.execute().await,
                };

                if let Some(result) = result {
                    _results_tx.send(result).unwrap();
                }
            }
        });

        self.cancellation_token = Some(cancellation_token);
        self.commands_tx = Some(commands_tx);
        self.results_rx = Some(results_rx);
        self.task = Some(task);
    }

    /// Cancels [`BgExecutor`] task.
    pub fn cancel(&mut self) {
        if let Some(cancellation_token) = self.cancellation_token.take() {
            cancellation_token.cancel();
            self.commands_tx = None;
            self.results_rx = None;
        }
    }

    /// Cancels [`BgExecutor`] task and waits until it is finished.
    pub fn stop(&mut self) {
        self.cancel();
        wait_for_task(self.task.take(), "executor");
    }

    /// Sends command to the [`BgExecutor`] to be executed in the background.
    pub fn run_command(&self, command: ExecutorCommand) {
        if let Some(commands_tx) = &self.commands_tx {
            commands_tx.send(command).unwrap();
        }
    }

    /// Tries to get next [`ExecutorResult`].
    pub fn try_next(&mut self) -> Option<ExecutorResult> {
        if let Some(results_rx) = &mut self.results_rx {
            results_rx.try_recv().ok()
        } else {
            None
        }
    }
}

impl Drop for BgExecutor {
    fn drop(&mut self) {
        self.cancel();
    }
}
