use tracing::error;

use crate::app::Config;

use super::CommandResult;

/// Command that saves provided configuration to a file.
pub struct SaveConfigurationCommand {
    pub config: Config,
}

impl SaveConfigurationCommand {
    /// Creates new [`SaveConfigurationCommand`] instance.
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    /// Saves configuration to a file.
    pub async fn execute(&self) -> Option<CommandResult> {
        if let Err(error) = self.config.save().await {
            error!("The configuration cannot be saved to a file: {}", error);
        }

        None
    }
}
