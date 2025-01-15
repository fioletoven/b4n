use crate::app::Config;

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
    pub async fn execute(&self) -> bool {
        self.config.save().await.is_ok()
    }
}
