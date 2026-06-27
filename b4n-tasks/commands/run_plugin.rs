use b4n_common::{DEFAULT_ERROR_DURATION, DEFAULT_MESSAGE_DURATION, NotificationSink};
use b4n_config::Plugin;
use b4n_kube::plugins::PluginContext;
use std::sync::Arc;
use tokio::process::Command;
use tokio::task::JoinSet;

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
        let once_index = if self.context.resources.len() == 1 { Some(0) } else { None };
        let for_each = self.plugin.for_each && self.context.resources.len() > 1;

        let plugin = Arc::new(self.plugin);
        let context = Arc::new(self.context);
        let footer_tx = self.footer_tx.clone();

        if for_each {
            execute_for_each(plugin, context, footer_tx).await;
        } else {
            execute_once(plugin, context, footer_tx, once_index).await;
        }

        None
    }
}

/// Executes plugin for all resources in parallel.
async fn execute_for_each(plugin: Arc<Plugin>, context: Arc<PluginContext>, footer_tx: NotificationSink) {
    let resource_count = context.resources.len();
    let mut join_set = JoinSet::new();

    for index in 0..resource_count {
        let plugin = Arc::clone(&plugin);
        let context = Arc::clone(&context);
        let footer_tx = footer_tx.clone();

        join_set.spawn(async move {
            execute_once(plugin, context, footer_tx, Some(index)).await;
        });
    }

    while let Some(result) = join_set.join_next().await {
        if let Err(error) = result {
            tracing::error!("Task panicked during plugin execution: {}", error);
        }
    }
}

/// Executes plugin for one resource or for all resources as one.
async fn execute_once(plugin: Arc<Plugin>, context: Arc<PluginContext>, footer_tx: NotificationSink, row_index: Option<usize>) {
    let resource_name = get_resource_name(&context, row_index);
    let resolved_args: Vec<String> = plugin.args.iter().map(|arg| context.resolve_arg(arg, row_index)).collect();

    let output = match Command::new(&plugin.command).args(&resolved_args).output().await {
        Ok(output) => output,
        Err(error) => {
            let msg = format!("Cannot execute '{}' ({}): {}", plugin.name, resource_name, error);
            tracing::error!("{}", msg);
            footer_tx.show_error(msg, DEFAULT_ERROR_DURATION);

            return;
        },
    };

    if output.status.success() {
        let msg = format!("'{}' ({}) executed successfully", plugin.name, resource_name);
        tracing::info!("{}", msg);
        footer_tx.show_info(msg, DEFAULT_MESSAGE_DURATION);

        return;
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let code = output.status.code().unwrap_or(-1);
    let msg = format!(
        "'{}' ({}) failed with exit code {}: {}",
        plugin.name,
        resource_name,
        code,
        stderr.trim()
    );

    tracing::error!("{}", msg);
    footer_tx.show_error(msg, DEFAULT_ERROR_DURATION);
}

fn get_resource_name(context: &Arc<PluginContext>, row_index: Option<usize>) -> String {
    if let Some(row_index) = row_index {
        context.resources.get(row_index).map_or_else(String::new, |r| {
            format!("{}/{}", r.namespace.as_str(), r.name.as_deref().unwrap_or_default())
        })
    } else {
        "all selected resources".to_string()
    }
}
