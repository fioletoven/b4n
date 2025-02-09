use tokio::{sync::mpsc::UnboundedSender, task::JoinHandle};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::app::{
    commands::{Command, CommandResult},
    utils::wait_for_task,
};

pub struct TaskResult {
    pub id: String,
    pub result: CommandResult,
}

/// Background task for background executor.
pub struct BgTask {
    uuid: String,
    command: Option<Command>,
    task: Option<JoinHandle<()>>,
    cancellation_token: Option<CancellationToken>,
}

impl BgTask {
    /// Creates new [`BgTask`] instance.  
    /// **Note** that it must be run in order to start execute a command.
    pub fn new(command: Command) -> Self {
        Self {
            uuid: Uuid::new_v4()
                .hyphenated()
                .encode_lower(&mut Uuid::encode_buffer())
                .to_owned(),
            command: Some(command),
            task: None,
            cancellation_token: None,
        }
    }

    /// Starts executing an associated command.
    pub fn run(&mut self, results_tx: UnboundedSender<TaskResult>) {
        let Some(_command) = self.command.take() else {
            return;
        };

        let cancellation_token = CancellationToken::new();
        let _cancellation_token = cancellation_token.clone();
        let _task_id = self.id().to_owned();

        let task = tokio::spawn(async move {
            tokio::select! {
                _ = _cancellation_token.cancelled() => (),
                result = run_command(_command) => {
                    if let Some(result) = result {
                        results_tx.send(TaskResult { id: _task_id, result }).unwrap();
                    }
                },
            }
        });

        self.task = Some(task);
        self.cancellation_token = Some(cancellation_token);
    }

    /// Unique task ID.
    pub fn id(&self) -> &str {
        &self.uuid
    }

    /// Indicates if the task is currently running.
    pub fn is_running(&self) -> bool {
        self.task.as_ref().is_some_and(|t| !t.is_finished())
    }

    /// Indicates if the task was started and is currently in a finished state.
    pub fn is_finished(&self) -> bool {
        self.command.is_none() && !self.is_running()
    }

    /// Cancels [`BgTask`] task.
    pub fn cancel(&mut self) {
        if let Some(cancellation_token) = self.cancellation_token.take() {
            cancellation_token.cancel();
        }
    }

    /// Cancels [`BgTask`] task and waits until it is finished.
    pub fn stop(&mut self) {
        self.cancel();
        wait_for_task(self.task.take(), "background command");
    }
}

/// Wrapper for running [`ExecutorCommand`].
async fn run_command(command: Command) -> Option<CommandResult> {
    match command {
        Command::ListKubeContexts(command) => command.execute().await,
        Command::NewKubernetesClient(command) => command.execute().await,
        Command::SaveConfiguration(command) => command.execute().await,
        Command::DeleteResource(mut command) => command.execute().await,
        Command::GetYaml(mut command) => command.execute().await,
    }
}
