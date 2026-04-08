use tokio::{runtime::Handle, sync::mpsc::UnboundedSender, task::JoinHandle};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::commands::{Command, CommandResult};

/// Result from the executed task.
pub struct TaskResult {
    pub id: String,
    pub result: CommandResult,
}

/// A task that has been created but not yet started.
pub struct PendingTask {
    pub id: String,
    pub command: Command,
    pub cancellation_token: CancellationToken,
}

impl PendingTask {
    /// Creates new [`PendingTask`] instance.
    pub fn new(command: Command, cancellation_token: CancellationToken) -> Self {
        Self {
            id: Uuid::new_v4()
                .hyphenated()
                .encode_lower(&mut Uuid::encode_buffer())
                .to_owned(),
            command,
            cancellation_token,
        }
    }
}

/// Background task for background executor.
pub struct BgTask {
    id: String,
    task: Option<JoinHandle<()>>,
    cancellation_token: Option<CancellationToken>,
}

impl BgTask {
    /// Creates and immediately starts executing the given [`PendingTask`].
    pub fn run(pending: PendingTask, runtime: &Handle, results_tx: UnboundedSender<Box<TaskResult>>) -> Self {
        let PendingTask {
            id,
            command,
            cancellation_token,
        } = pending;

        let token = cancellation_token.clone();
        let task_id = id.clone();

        let task = runtime.spawn(async move {
            tokio::select! {
                () = token.cancelled() => (),
                result = run_command(command) => {
                    if let Some(result) = result
                        && let Err(error) = results_tx.send(Box::new(TaskResult { id: task_id, result }))
                    {
                        tracing::warn!("Cannot send task result: {}", error);
                    }
                },
            }
        });

        Self {
            id,
            task: Some(task),
            cancellation_token: Some(cancellation_token),
        }
    }

    /// Unique task ID.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Indicates if the task is currently running.
    pub fn is_running(&self) -> bool {
        self.task.as_ref().is_some_and(|t| !t.is_finished())
    }

    /// Indicates if the task is currently in a finished state.
    pub fn is_finished(&self) -> bool {
        !self.is_running()
    }

    /// Cancels [`BgTask`] task.
    pub fn cancel(&mut self) {
        if let Some(token) = self.cancellation_token.take() {
            token.cancel();
        }
    }

    /// Cancels [`BgTask`] task and waits until it is finished.
    pub fn stop(&mut self) {
        self.cancel();
        b4n_common::tasks::wait_for_task(self.task.take(), "background command");
    }
}

/// Wrapper for running [`ExecutorCommand`].
pub async fn run_command(command: Command) -> Option<CommandResult> {
    match command {
        Command::ListKubeContexts(command) => command.execute().await,
        Command::ListThemes(command) => command.execute().await,
        Command::ListResourcePorts(command) => command.execute().await,
        Command::NewKubernetesClient(command) => command.execute().await,
        Command::SaveConfig(command) => command.execute().await,
        Command::SaveHistory(command) => command.execute().await,
        Command::DeleteResource(command) => command.execute().await,
        Command::GetNewYaml(command) => command.execute().await,
        Command::GetYaml(command) => command.execute().await,
        Command::SetNewYaml(command) => command.execute().await,
        Command::SetYaml(command) => command.execute().await,
    }
}
