use tokio::{
    sync::mpsc::{self, UnboundedSender},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;

use crate::{app::utils::wait_for_task, kubernetes::client::KubernetesClient};

use super::Command;

/// Background commands executor
#[derive(Default)]
pub struct BgExecutor {
    task: Option<JoinHandle<()>>,
    cancellation_token: Option<CancellationToken>,
    commands_tx: Option<UnboundedSender<Command>>,
}

impl BgExecutor {
    /// Starts background task for commands execution
    pub fn start(&mut self, client: &KubernetesClient) {
        if self.cancellation_token.is_some() {
            return;
        }

        let cancellation_token = CancellationToken::new();
        let _cancellation_token = cancellation_token.clone();

        let (commands_tx, mut _commands_rx) = mpsc::unbounded_channel();
        let _client = client.get_client();

        let task = tokio::spawn(async move {
            while !_cancellation_token.is_cancelled() {
                let command = tokio::select! {
                    _ = _cancellation_token.cancelled() => break,
                    v = _commands_rx.recv() => v,
                };

                let Some(command) = command else {
                    break;
                };

                match command {
                    Command::SaveConfiguration(command) => command.execute().await,
                    Command::DeleteResource(mut command) => command.execute(&_client).await,
                };
            }
        });

        self.cancellation_token = Some(cancellation_token);
        self.commands_tx = Some(commands_tx);
        self.task = Some(task);
    }

    /// Cancels [`BgExecutor`] task
    pub fn cancel(&mut self) {
        if let Some(cancellation_token) = self.cancellation_token.take() {
            cancellation_token.cancel();
        }
    }

    /// Cancels [`BgExecutor`] task and waits until it is finished
    pub fn stop(&mut self) {
        if let Some(cancellation_token) = self.cancellation_token.take() {
            cancellation_token.cancel();
            wait_for_task(self.task.take(), "executor");
        }
    }

    /// Sends command to the [`BgExecutor`] to be executed in the background
    pub fn run_command(&self, command: Command) {
        if let Some(commands_tx) = &self.commands_tx {
            commands_tx.send(command).unwrap();
        }
    }
}

impl Drop for BgExecutor {
    fn drop(&mut self) {
        self.cancel();
    }
}
