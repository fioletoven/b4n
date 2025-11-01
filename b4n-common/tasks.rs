use std::time::Duration;
use tokio::task::JoinHandle;

/// Synchronously waits for task to end (e.g. after cancellation).
pub fn wait_for_task<T>(task: Option<JoinHandle<T>>, task_name: &str) {
    let Some(task) = task else {
        return;
    };

    let mut counter = 0;
    while !task.is_finished() {
        std::thread::sleep(Duration::from_millis(1));

        counter += 1;

        if counter > 50 {
            task.abort();
        }

        if counter > 100 {
            tracing::error!("Failed to abort {task_name} task in 100 milliseconds for an unknown reason.");
            break;
        }
    }
}
