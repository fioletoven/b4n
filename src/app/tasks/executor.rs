use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

use crate::app::commands::Command;

use super::{BgTask, TaskResult};

/// Background commands executor.
pub struct BgExecutor {
    tasks: Vec<BgTask>,
    results_tx: UnboundedSender<TaskResult>,
    results_rx: UnboundedReceiver<TaskResult>,
}

impl Default for BgExecutor {
    fn default() -> Self {
        let (results_tx, results_rx) = unbounded_channel();
        Self {
            tasks: Vec::new(),
            results_tx,
            results_rx,
        }
    }
}

impl BgExecutor {
    /// Creates a task with the specified command and runs it.  
    /// **Note** that it returns a unique task ID by which the task can be cancelled.
    pub fn run_task(&mut self, command: Command) -> String {
        let mut task = BgTask::new(command);
        task.run(self.results_tx.clone());
        let id = task.id().to_owned();
        self.tasks.push(task);
        self.cleanup_finished();

        id
    }

    /// Cancels the task specified by its unique ID.
    pub fn cancel_task(&mut self, id: &str) -> bool {
        let Some(index) = self.tasks.iter().position(|t| t.id() == id) else {
            return false;
        };

        let mut task = self.tasks.remove(index);
        let is_running = task.is_running();
        task.cancel();
        self.cleanup_finished();

        is_running
    }

    /// Removes from the internal list of tasks all finished tasks.
    pub fn cleanup_finished(&mut self) {
        self.tasks.retain(|t| !t.is_finished());
    }

    /// Cancels all currently running tasks.
    pub fn cancel_all(&mut self) {
        for task in &mut self.tasks {
            task.cancel();
        }

        self.tasks.clear();
    }

    /// Cancels all currently running tasks and waits for them to finish.  
    /// **Note** that it can be a slow operation. It stops tasks one by one.
    pub fn stop_all(&mut self) {
        for task in &mut self.tasks {
            task.stop();
        }

        self.tasks.clear();
    }

    /// Tries to get the next [`ExecutorResult`].
    pub fn try_next(&mut self) -> Option<TaskResult> {
        self.results_rx.try_recv().ok()
    }
}

impl Drop for BgExecutor {
    fn drop(&mut self) {
        self.cancel_all();
    }
}
