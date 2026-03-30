use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::runtime::Handle;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::tasks::run_command;
use crate::{BgTask, TaskResult, commands::Command};

type SequentialTask = (String, Command, CancellationToken);

/// Background commands executor.
pub struct BgExecutor {
    runtime: Handle,
    tasks: Vec<BgTask>,
    results_tx: UnboundedSender<Box<TaskResult>>,
    results_rx: UnboundedReceiver<Box<TaskResult>>,
    sequential_worker: Option<JoinHandle<()>>,
    sequential_token: CancellationToken,
    sequential_drain: Arc<AtomicBool>,
    sequential_tx: UnboundedSender<SequentialTask>,
}

impl BgExecutor {
    /// Creates new [`BgExecutor`] instance.
    pub fn new(runtime: Handle) -> Self {
        let (results_tx, results_rx) = unbounded_channel();
        let (sequential_tx, sequential_rx) = unbounded_channel();

        let sequential_token = CancellationToken::new();
        let sequential_drain = Arc::new(AtomicBool::new(false));

        let worker_results_tx = results_tx.clone();
        let worker_drain = sequential_drain.clone();
        let sequential_worker = runtime.spawn(Self::sequential_worker(sequential_rx, worker_results_tx, worker_drain));

        Self {
            runtime,
            tasks: Vec::new(),
            results_tx,
            results_rx,
            sequential_worker: Some(sequential_worker),
            sequential_token,
            sequential_drain,
            sequential_tx,
        }
    }

    /// Worker that processes commands one by one in order.
    async fn sequential_worker(
        mut rx: UnboundedReceiver<SequentialTask>,
        results_tx: UnboundedSender<Box<TaskResult>>,
        drain: Arc<AtomicBool>,
    ) {
        while let Some((task_id, command, cancellation_token)) = rx.recv().await {
            if drain.load(Ordering::Relaxed) {
                while rx.try_recv().is_ok() {}
                drain.store(false, Ordering::Relaxed);
                continue;
            }

            let result = tokio::select! {
                () = cancellation_token.cancelled() => {
                    if drain.load(Ordering::Relaxed) {
                        while rx.try_recv().is_ok() {}
                        drain.store(false, Ordering::Relaxed);
                    }
                    continue;
                },
                result = run_command(command) => result,
            };

            if let Some(result) = result
                && let Err(error) = results_tx.send(Box::new(TaskResult { id: task_id, result }))
            {
                tracing::warn!("Cannot send sequential task result: {}", error);
            }
        }
    }

    /// Creates a task with the specified command and runs it.\
    /// **Note** that it returns a unique task ID by which the task can be canceled.
    pub fn run_task(&mut self, command: Command) -> String {
        if command.is_sequential() {
            return self.enqueue_sequential(command);
        }

        let mut task = BgTask::new(command);
        task.run(&self.runtime, self.results_tx.clone());
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
        self.cancel_sequential();
    }

    /// Cancels all currently running tasks and waits for them to finish.\
    /// **Note** that it can be a slow operation. It stops tasks one by one.
    pub fn stop_all(&mut self) {
        for task in &mut self.tasks {
            task.stop();
        }

        self.tasks.clear();
        self.cancel_sequential();
    }

    /// Tries to get the next [`TaskResult`].
    pub fn try_next(&mut self) -> Option<Box<TaskResult>> {
        self.results_rx.try_recv().ok()
    }

    /// Enqueues a command for sequential execution.
    fn enqueue_sequential(&self, command: Command) -> String {
        let id = uuid::Uuid::new_v4()
            .hyphenated()
            .encode_lower(&mut uuid::Uuid::encode_buffer())
            .to_owned();

        let token = self.sequential_token.clone();
        if let Err(error) = self.sequential_tx.send((id.clone(), command, token)) {
            tracing::warn!("Cannot enqueue sequential command: {}", error);
        }

        id
    }

    /// Signals the sequential worker to cancel the current command and drain all pending ones.
    fn cancel_sequential(&mut self) {
        self.sequential_drain.store(true, Ordering::Release);
        self.sequential_token.cancel();
        self.sequential_token = CancellationToken::new();
    }
}

impl Drop for BgExecutor {
    fn drop(&mut self) {
        self.cancel_all();

        if let Some(worker) = self.sequential_worker.take() {
            worker.abort();
        }
    }
}
