use b4n_common::{DEFAULT_ERROR_DURATION, DEFAULT_MESSAGE_DURATION, NotificationSink};
use b4n_config::themes::YamlSyntaxColors;
use b4n_config::{Plugin, PluginOutputType};
use b4n_kube::plugins::PluginContext;
use ratatui_core::style::Style;
use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::mpsc::UnboundedSender;
use tokio::task::JoinSet;

use crate::commands::CommandResult;
use crate::{HighlightRequest, HighlightResponse, highlight_yaml};

/// Indicates error while running command.\
/// Used only to signal that view should be closed.
pub struct RunPluginError;

/// Result for the [`RunPluginCommand`] command.
pub struct RunPluginOutput {
    pub output: Vec<String>,
    pub styled: Vec<Vec<(Style, String)>>,
}

/// Command that executes an external binary with resolved arguments from the plugin context.
pub struct RunPluginCommand {
    plugin: Plugin,
    context: PluginContext,
    highlighter: UnboundedSender<HighlightRequest>,
    colors: YamlSyntaxColors,
    footer_tx: NotificationSink,
}

impl RunPluginCommand {
    /// Creates new [`RunPluginCommand`] instance.
    pub fn new(
        plugin: Plugin,
        context: PluginContext,
        highlighter: UnboundedSender<HighlightRequest>,
        colors: YamlSyntaxColors,
        footer_tx: NotificationSink,
    ) -> Self {
        Self {
            plugin,
            context,
            highlighter,
            colors,
            footer_tx,
        }
    }

    /// Resolves arguments using the plugin context and executes the binary.
    pub async fn execute(self) -> Option<CommandResult> {
        let once_index = if self.context.resources.len() == 1 { Some(0) } else { None };
        let keep_output = self.plugin.keep_output;
        let for_each = self.plugin.for_each && self.context.resources.len() > 1;

        let plugin = Arc::new(self.plugin);
        let context = Arc::new(self.context);
        let footer_tx = self.footer_tx.clone();
        let highlighter = self.highlighter;

        if keep_output {
            let result = execute_output(plugin, context, footer_tx, highlighter, self.colors, once_index).await;
            Some(CommandResult::RunPluginOutput(result))
        } else if for_each {
            execute_for_each(plugin, context, footer_tx).await;
            None
        } else {
            execute_once(plugin, context, footer_tx, once_index).await;
            None
        }
    }
}

/// Runs the command, captures stdout and highlights it.
async fn execute_output(
    plugin: Arc<Plugin>,
    context: Arc<PluginContext>,
    footer_tx: NotificationSink,
    highlighter: UnboundedSender<HighlightRequest>,
    colors: YamlSyntaxColors,
    row_index: Option<usize>,
) -> Result<RunPluginOutput, RunPluginError> {
    let resource_name = get_resource_name(&context, row_index);
    let resolved_args: Vec<String> = plugin.args.iter().map(|arg| context.resolve_arg(arg, row_index)).collect();

    let mut command = Command::new(&plugin.command);
    command.args(&resolved_args);
    if let Some(current_dir) = &plugin.current_dir {
        command.current_dir(current_dir);
    }

    let output = match command.output().await {
        Ok(output) => output,
        Err(error) => {
            let msg = format!("Cannot execute '{}' ({}): {}", plugin.name, resource_name, error);
            tracing::error!("{}", msg);
            footer_tx.show_error(msg, DEFAULT_ERROR_DURATION);

            return Err(RunPluginError);
        },
    };

    let raw = if output.status.success() {
        String::from_utf8_lossy(&output.stdout).into_owned()
    } else {
        show_error(&plugin.name, &resource_name, &output, &footer_tx);
        return Err(RunPluginError);
    };

    if plugin.output_type == PluginOutputType::Plain {
        let plain = raw.lines().map(String::from).collect::<Vec<_>>();
        let styled = plain
            .iter()
            .map(|l| vec![((&colors.string).into(), l.clone())])
            .collect::<Vec<_>>();
        Ok(RunPluginOutput { output: plain, styled })
    } else {
        match highlight_yaml(&highlighter, raw).await {
            Ok(result) => Ok(process_highlight_result(
                result,
                &colors,
                plugin.output_type == PluginOutputType::Describe,
            )),
            Err(error) => {
                let msg = format!("'{}' ({}) cannot highlight output: {}", plugin.name, resource_name, error);
                tracing::error!("{}", msg);
                footer_tx.show_error(msg, DEFAULT_ERROR_DURATION);

                Err(RunPluginError)
            },
        }
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

    let mut command = Command::new(&plugin.command);
    command.args(&resolved_args);
    if let Some(current_dir) = &plugin.current_dir {
        command.current_dir(current_dir);
    }

    let output = match command.output().await {
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
    } else {
        show_error(&plugin.name, &resource_name, &output, &footer_tx);
    }
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

fn show_error(plugin_name: &str, resource_name: &str, output: &std::process::Output, footer_tx: &NotificationSink) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let code = output.status.code().unwrap_or(-1);
    let msg = format!(
        "'{}' ({}) failed with exit code {}: {}",
        plugin_name,
        resource_name,
        code,
        stderr.trim()
    );
    tracing::error!("{}", msg);
    footer_tx.show_error(msg, DEFAULT_ERROR_DURATION);
}

fn process_highlight_result(mut result: HighlightResponse, colors: &YamlSyntaxColors, is_describe: bool) -> RunPluginOutput {
    if is_describe {
        fix_describe_result(&mut result, colors);
    }

    RunPluginOutput {
        output: result.plain,
        styled: result.styled,
    }
}

/// Fixes YAML highlight result to match `kubectl describe` output.
fn fix_describe_result(result: &mut HighlightResponse, colors: &YamlSyntaxColors) {
    let mut plain_mode = false;
    let mut last_key_indent: Option<usize> = None;

    for (line, plain) in result.styled.iter_mut().zip(result.plain.iter()) {
        if plain_mode {
            for (style, _) in line.iter_mut() {
                *style = (&colors.string).into();
            }
            continue;
        }

        if plain.starts_with("Events:") {
            plain_mode = true;
            continue;
        }

        let current_indent = plain.len() - plain.trim_start().len();

        let is_continuation = last_key_indent.is_some_and(|key_indent| current_indent > key_indent + 2);
        if is_continuation {
            fix_continuation_line_spans(line, colors);
        } else {
            let has_property = fix_line_spans(line, colors);
            if has_property {
                last_key_indent = Some(current_indent);
            }
        }
    }
}

/// Recolors all spans that are colored as properties if they are after the first property color.\
/// Returns `true` if there was at least one span with the property color.
fn fix_line_spans(line: &mut [(Style, String)], colors: &YamlSyntaxColors) -> bool {
    let mut has_property = false;
    let mut has_colon = false;

    for (style, text) in line {
        if has_property {
            if has_colon {
                if *style == colors.property || (*style == colors.normal && text.trim() == ":") {
                    *style = (&colors.string).into();
                }
            } else if *style == colors.normal && text.trim() == ":" {
                has_colon = true;
            }
        } else if *style == colors.property {
            has_property = true;
        }
    }

    has_property && has_colon
}

/// Recolors all spans that are colored as properties.
fn fix_continuation_line_spans(line: &mut [(Style, String)], colors: &YamlSyntaxColors) {
    for (style, text) in line.iter_mut() {
        if *style == colors.property || (*style == colors.normal && text.trim() == ":") {
            *style = (&colors.string).into();
        }
    }
}
