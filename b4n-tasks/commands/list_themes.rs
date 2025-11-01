use b4n_config::Config;
use std::path::PathBuf;
use tokio::fs;

use crate::commands::CommandResult;

/// Command that lists all available files in the themes directory.
pub struct ListThemesCommand;

impl ListThemesCommand {
    /// Gets all files from the themes directory.
    pub async fn execute(&self) -> Option<CommandResult> {
        if let Ok(list) = get_themes().await {
            Some(CommandResult::ThemesList(list))
        } else {
            None
        }
    }
}

async fn get_themes() -> Result<Vec<PathBuf>, std::io::Error> {
    let mut result = Vec::new();
    let path = Config::themes_dir();
    let mut dir = fs::read_dir(path).await?;
    while let Some(entry) = dir.next_entry().await? {
        let file_type = entry.file_type().await?;
        if file_type.is_file() {
            result.push(entry.path());
        }
    }

    Ok(result)
}
