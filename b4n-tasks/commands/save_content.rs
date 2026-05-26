use b4n_common::{DEFAULT_ERROR_DURATION, DEFAULT_MESSAGE_DURATION, NotificationSink};
use std::path::PathBuf;
use tokio::fs::{self, File};
use tokio::io::AsyncWriteExt;

use crate::commands::CommandResult;

/// Command that saves specified content to a file.
pub struct SaveContentCommand {
    path: PathBuf,
    content: String,
    footer_tx: NotificationSink,
}

impl SaveContentCommand {
    /// Creates new [`SaveContentCommand`] instance.
    pub fn new(path: PathBuf, content: String, footer_tx: NotificationSink) -> Self {
        Self {
            path,
            content,
            footer_tx,
        }
    }

    /// Saves content to the specified file.
    pub async fn execute(self) -> Option<CommandResult> {
        if let Some(parent) = self.path.parent()
            && let Err(error) = fs::create_dir_all(parent).await
        {
            let msg = format!("Cannot create directories for {}: {}", self.path.display(), error);
            tracing::error!("{}", msg);
            self.footer_tx.show_error(msg, DEFAULT_ERROR_DURATION);

            return None;
        }

        match File::create(&self.path).await {
            Ok(mut file) => {
                if let Err(error) = file.write_all(self.content.as_bytes()).await {
                    let msg = format!("Cannot write content to {}: {}", self.path.display(), error);
                    tracing::error!("{}", msg);
                    self.footer_tx.show_error(msg, DEFAULT_ERROR_DURATION);

                    return None;
                }

                if let Err(error) = file.flush().await {
                    let msg = format!("Cannot flush file {}: {}", self.path.display(), error);
                    tracing::error!("{}", msg);
                    self.footer_tx.show_error(msg, DEFAULT_ERROR_DURATION);

                    return None;
                }

                self.footer_tx
                    .show_info(format!("Content saved to: {}", self.path.display()), DEFAULT_MESSAGE_DURATION);
            },
            Err(error) => {
                let msg = format!("Cannot create content file {}: {}", self.path.display(), error);
                tracing::error!("{}", msg);
                self.footer_tx.show_error(msg, DEFAULT_ERROR_DURATION);
            },
        }

        None
    }
}
