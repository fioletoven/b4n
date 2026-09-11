use b4n_config::Config;

use crate::commands::CommandResult;

/// Possible errors from copying missing resources to the data directory.
#[derive(thiserror::Error, Debug)]
pub enum InstallationError {
    #[error("cannot determine executable path")]
    ExecutablePathError,

    #[error("cannot determine executable directory")]
    ExecutableDirError,

    #[error("io error while installing resources: {0}")]
    IoError(#[from] std::io::Error),
}

/// Represents success installation result that contains theme name to switch to.
pub struct InstallationResult {
    pub theme: Option<String>,
}

/// Command that installs missing resource directories (themes, plugins) from the bundled assets
/// next to the executable.
pub struct InstallMissingResourcesCommand {
    prefer_dark_theme: bool,
    prefer_light_theme: bool,
}

impl InstallMissingResourcesCommand {
    /// Creates new [`InstallMissingResourcesCommand`] instance.
    pub fn new(prefer_dark_theme: bool, prefer_light_theme: bool) -> Self {
        Self {
            prefer_dark_theme,
            prefer_light_theme,
        }
    }

    /// Copies missing resources to the `themes` and `plugins` directories from the bundled assets.
    pub async fn execute(&self) -> Option<CommandResult> {
        let result = match self.install_resources().await {
            Ok(theme) => Ok(InstallationResult {
                theme: theme.map(String::from),
            }),
            Err(error) => Err(error),
        };

        Some(CommandResult::InstallMissingResources(result))
    }

    async fn install_resources(&self) -> Result<Option<&'static str>, InstallationError> {
        let exe_path = std::env::current_exe().map_err(|_| InstallationError::ExecutablePathError)?;
        let exe_dir = exe_path.parent().ok_or(InstallationError::ExecutableDirError)?;

        copy_dir(&exe_dir.join("themes"), &Config::themes_dir()).await?;
        copy_dir(&exe_dir.join("plugins"), &Config::plugins_dir()).await?;

        Ok(self.resolve_theme())
    }

    fn resolve_theme(&self) -> Option<&'static str> {
        let themes_dir = Config::themes_dir();

        if self.prefer_dark_theme && matches!(themes_dir.join("dark-fixed.yaml").try_exists(), Ok(true)) {
            return Some("dark-fixed");
        }

        if self.prefer_light_theme && matches!(themes_dir.join("light-fixed.yaml").try_exists(), Ok(true)) {
            return Some("light-fixed");
        }

        None
    }
}

async fn copy_dir(source: &std::path::Path, target: &std::path::Path) -> Result<(), InstallationError> {
    if let Ok(true) = source.try_exists() {
        tokio::fs::create_dir_all(target).await?;

        let mut entries = tokio::fs::read_dir(source).await?;
        while let Some(entry) = entries.next_entry().await? {
            let entry_path = entry.path();
            if let Ok(true) = entry_path.try_exists()
                && entry_path.is_file()
            {
                let target_path = target.join(entry.file_name());
                tokio::fs::copy(&entry_path, &target_path).await?;
            }
        }
    }

    Ok(())
}
