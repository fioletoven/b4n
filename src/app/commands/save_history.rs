use tracing::error;

use crate::app::{History, Persistable};

use super::CommandResult;

/// Command that saves provided app history data to a file.
pub struct SaveHistoryCommand {
    pub history: History,
}

impl SaveHistoryCommand {
    /// Creates new [`SaveHistoryCommand`] instance.
    pub fn new(history: History) -> Self {
        Self { history }
    }

    /// Saves app history data to a file.
    pub async fn execute(&self) -> Option<CommandResult> {
        if let Err(error) = self.history.save().await {
            error!("The history data cannot be saved to a file: {}", error);
        }

        None
    }
}
