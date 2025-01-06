use crate::app::Config;

/// Command that saves provided configuration to a file
pub struct SaveConfigCommand {
    pub config: Config,
}

impl SaveConfigCommand {
    /// Creates new [`SaveConfigCommand`] instance
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    /// Saves configuration to a file
    pub async fn execute(&self) -> bool {
        self.config.save().await.is_ok()
    }
}
