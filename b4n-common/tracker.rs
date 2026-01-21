use std::time::{Duration, Instant};

/// Tracks state change.
pub struct StateChangeTracker<T: PartialEq> {
    last_state: Option<T>,
}

impl<T: Default + PartialEq> Default for StateChangeTracker<T> {
    fn default() -> Self {
        Self { last_state: None }
    }
}

impl<T: PartialEq> StateChangeTracker<T> {
    /// Creates new [`StateChangeTracker`] instance.
    pub fn new(initial_state: Option<T>) -> Self {
        Self {
            last_state: initial_state,
        }
    }

    /// Sets new state and returns it if changed.
    pub fn changed(&mut self, new_state: T) -> Option<&T> {
        if self.last_state.as_ref().is_none_or(|last_state| *last_state != new_state) {
            self.last_state = Some(new_state);
            self.last_state.as_ref()
        } else {
            None
        }
    }
}

/// Tracks state change with the given delay.
pub struct DelayedTrueTracker {
    last_true_time: Option<Instant>,
    delay: Duration,
    current_value: bool,
}

impl Default for DelayedTrueTracker {
    fn default() -> Self {
        Self {
            last_true_time: None,
            delay: Duration::from_millis(500),
            current_value: false,
        }
    }
}

impl DelayedTrueTracker {
    /// Creates new [`DelayedTrueTracker`] instance.
    pub fn new(delay: Duration) -> Self {
        Self {
            last_true_time: None,
            delay,
            current_value: false,
        }
    }

    /// Returns current [`DelayedTrueTracker`] value.
    pub fn value(&self) -> bool {
        match self.last_true_time {
            Some(t) => self.current_value && t.elapsed() >= self.delay,
            None => false,
        }
    }

    /// Updates [`DelayedTrueTracker`] value to the specified one.
    pub fn update(&mut self, value: bool) -> bool {
        if value && !self.current_value {
            self.last_true_time = Some(Instant::now());
        }

        self.current_value = value;
        self.value()
    }
}
