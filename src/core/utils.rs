use backoff::{ExponentialBackoff, ExponentialBackoffBuilder};
use std::time::Duration;
use tokio::task::JoinHandle;
use tracing::error;

/// Returns default exponential backoff policy.
pub fn build_default_backoff() -> ExponentialBackoff {
    ExponentialBackoffBuilder::new()
        .with_initial_interval(Duration::from_millis(800))
        .with_max_interval(Duration::from_secs(30))
        .with_randomization_factor(1.0)
        .with_multiplier(2.0)
        .with_max_elapsed_time(None)
        .build()
}

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
            error!("Failed to abort {task_name} task in 100 milliseconds for an unknown reason.");
            break;
        }
    }
}

/// Tracks state change.
pub struct StateChangeTracker<T: PartialEq> {
    last_state: T,
}

impl<T: Default + PartialEq> Default for StateChangeTracker<T> {
    fn default() -> Self {
        Self {
            last_state: Default::default(),
        }
    }
}

impl<T: PartialEq> StateChangeTracker<T> {
    /// Creates new [`StateChangeTracker`] instance.
    pub fn new(initial_state: T) -> Self {
        Self {
            last_state: initial_state,
        }
    }

    /// Sets new state and returns `true` if it changed from the last time.
    pub fn changed(&mut self, new_state: T) -> bool {
        let state_changed = self.last_state != new_state;
        self.last_state = new_state;
        state_changed
    }

    /// Sets new state and returns `true` if it changed to the `check` from the last time.
    pub fn changed_to(&mut self, new_state: T, check: &T) -> bool {
        if self.changed(new_state) {
            self.last_state == *check
        } else {
            false
        }
    }
}
