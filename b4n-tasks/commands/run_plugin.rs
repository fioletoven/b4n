use b4n_common::{DEFAULT_ERROR_DURATION, DEFAULT_MESSAGE_DURATION, NotificationSink};
use b4n_config::Plugin;
use b4n_kube::plugins::PluginContext;
use tokio::process::Command;

use crate::commands::CommandResult;

/// Command that executes an external binary with resolved arguments from the plugin context.
pub struct RunPluginCommand {
    plugin: Plugin,
    context: PluginContext,
    footer_tx: NotificationSink,
}

impl RunPluginCommand {
    /// Creates new [`RunPluginCommand`] instance.
    pub fn new(plugin: Plugin, context: PluginContext, footer_tx: NotificationSink) -> Self {
        Self {
            plugin,
            context,
            footer_tx,
        }
    }

    /// Resolves arguments using the plugin context and executes the binary.
    pub async fn execute(self) -> Option<CommandResult> {
        if self.plugin.for_each && !self.context.resources.is_empty() {
            self.execute_for_each().await;
        } else {
            self.execute_once(None).await;
        }

        None
    }

    async fn execute_for_each(self) -> bool {
        let resource_count = self.context.resources.len();
        let mut any_success = false;
        let mut any_failure = false;

        for index in 0..resource_count {
            if self.execute_once(Some(index)).await {
                any_success = true;
            } else {
                any_failure = true;

                if self.plugin.stop_on_error {
                    tracing::warn!(
                        "Stopping execution of '{}' on first error (current index: {}).",
                        &self.plugin.name,
                        index
                    );
                    break;
                }
            }
        }

        any_success && !any_failure
    }

    async fn execute_once(&self, row_index: Option<usize>) -> bool {
        let resource_name = if let Some(row_index) = row_index {
            self.context
                .resources
                .get(row_index)
                .map(|r| format!("{}/{}", r.namespace.as_str(), r.name.as_deref().unwrap_or_default()))
                .unwrap_or_else(String::new)
        } else {
            "all selected resources".to_string()
        };

        let resolved_args: Vec<String> = self
            .plugin
            .args
            .iter()
            .map(|arg| self.context.resolve_arg(arg, row_index))
            .collect();

        tracing::debug!(
            binary = %self.plugin.command,
            args = ?resolved_args,
            "Executing plugin command"
        );

        let output = match Command::new(&self.plugin.command).args(&resolved_args).output().await {
            Ok(output) => output,
            Err(error) => {
                let msg = format!("Cannot execute '{}' ({}): {}", self.plugin.name, resource_name, error);
                tracing::error!("{}", msg);
                self.footer_tx.show_error(msg, DEFAULT_ERROR_DURATION);

                return false;
            },
        };

        if output.status.success() {
            let msg = format!("'{}' ({}) executed successfully", self.plugin.name, resource_name);
            tracing::info!("{}", msg);
            self.footer_tx.show_info(msg, DEFAULT_MESSAGE_DURATION);

            return true;
        }

        let stderr = String::from_utf8_lossy(&output.stderr);
        let code = output.status.code().unwrap_or(-1);
        let msg = format!(
            "'{}' ({}) failed with exit code {}: {}",
            self.plugin.name,
            resource_name,
            code,
            stderr.trim()
        );

        tracing::error!("{}", msg);
        self.footer_tx.show_error(msg, DEFAULT_ERROR_DURATION);

        false
    }
}
