use b4n_config::Persistable;
use tracing::error;

use crate::commands::CommandResult;

/// Command that saves provided configuration data to a file.
pub struct SaveConfigurationCommand<T: Persistable<T>> {
    pub config: T,
}

impl<T: Persistable<T>> SaveConfigurationCommand<T> {
    /// Creates new [`SaveConfigurationCommand`] instance.
    pub fn new(config: T) -> Self {
        Self { config }
    }

    /// Saves app configuration data to a file.
    pub async fn execute(&self) -> Option<CommandResult> {
        if let Err(error) = self.config.save(&T::default_path()).await {
            error!("The configuration data cannot be saved to a file: {}", error);
        }

        None
    }
}
