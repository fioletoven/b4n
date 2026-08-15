use b4n_config::{Config, Persistable, themes::Theme};
use std::path::PathBuf;
use tokio::fs;

use crate::commands::CommandResult;

/// Command that lists all available files in the themes directory.
pub struct ListThemesCommand;

impl ListThemesCommand {
    /// Gets all files from the themes directory.\
    /// **Note** that it includes `default` theme if not present.
    pub async fn execute(&self) -> Option<CommandResult> {
        let default = Theme::default_path();
        let path = Config::themes_dir();
        let mut list = list_themes(&path).await.unwrap_or_default();
        if !list.contains(&default) {
            list.push(default);
        }

        Some(CommandResult::ThemesList(list))
    }
}

async fn list_themes(path: &PathBuf) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut result = Vec::new();
    let mut dir = fs::read_dir(path).await?;

    while let Some(entry) = dir.next_entry().await? {
        let file_type = entry.file_type().await?;
        if file_type.is_file() {
            let path = entry.path();
            if let Some(extension) = path.extension()
                && (extension.eq_ignore_ascii_case("yaml") || extension.eq_ignore_ascii_case("yml"))
            {
                result.push(path);
            }
        }
    }

    Ok(result)
}
