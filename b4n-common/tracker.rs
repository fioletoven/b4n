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

    /// Sets new state and returns `true` if it changed to the `check` value.
    pub fn changed_to(&mut self, new_state: T, check: &T) -> bool {
        if self.changed(new_state).is_some() {
            self.last_state.as_ref().is_some_and(|last_state| *last_state == *check)
        } else {
            false
        }
    }
}
