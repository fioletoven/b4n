use b4n_common::expr::{Expression, ExpressionExt, SelectiveMap, parse};
use b4n_common::truncate;
use b4n_config::themes::{TextColors, Theme};
use b4n_kube::ContainerRef;
use b4n_list::{FilterContext, Filterable, Row};
use b4n_tasks::PortForwardTask;
use k8s_openapi::jiff::Timestamp;
use std::{borrow::Cow, sync::atomic::Ordering};

/// Represents port forward list item.
pub struct PortForwardItem {
    pub uid: String,
    group: String,
    name: String,
    container: Option<String>,
    age: Option<String>,
    creation_timestamp: Option<Timestamp>,
    bind_address: String,
    port: String,
    port_sort: String,
    overall: String,
    overall_sort: String,
    active: String,
    active_sort: String,
    errors: String,
    errors_sort: String,
    filter_metadata: SelectiveMap,
}

impl PortForwardItem {
    /// Creates new [`PortForwardItem`] instance.
    pub fn from(task: &PortForwardTask) -> Self {
        let overall = task.statistics.overall_connections.load(Ordering::Relaxed);
        let active = task.statistics.active_connections.load(Ordering::Relaxed);
        let errors = task.statistics.connection_errors.load(Ordering::Relaxed);
        let filter_metadata = get_filter_metadata(task);

        Self {
            uid: task.uuid.clone(),
            group: task.resource.namespace.as_str().to_owned(),
            name: task.resource.name.as_deref().unwrap_or_default().to_owned(),
            container: task.resource.container.clone(),
            age: task.start_time.as_ref().map(|t| t.as_millisecond().to_string()),
            creation_timestamp: task.start_time,
            bind_address: task.bind_address.clone(),
            port: task.port.to_string(),
            port_sort: format!("{:0>6}", task.port),
            overall: overall.to_string(),
            overall_sort: format!("{overall:0>6}"),
            active: active.to_string(),
            active_sort: format!("{active:0>6}"),
            errors: errors.to_string(),
            errors_sort: format!("{errors:0>6}"),
            filter_metadata,
        }
    }

    /// Returns [`TextColors`] for this port forward item considering `theme` and other data.
    pub fn get_colors(&self, theme: &Theme, is_active: bool, is_selected: bool) -> TextColors {
        theme.colors.line.ready.get_specific(is_active, is_selected)
    }

    /// Returns container name.
    pub fn container(&self) -> Option<&str> {
        self.container.as_deref()
    }
}

impl Row for PortForwardItem {
    fn uid(&self) -> &str {
        &self.uid
    }

    fn group(&self) -> &str {
        &self.group
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn creation_timestamp(&self) -> Option<&Timestamp> {
        self.creation_timestamp.as_ref()
    }

    fn get_name(&self, width: usize) -> String {
        format!("{1:<0$}", width, truncate(self.name.as_str(), width))
    }

    fn column_text(&self, column: usize) -> Cow<'_, str> {
        Cow::Borrowed(match column {
            0 => self.group(),
            1 => self.name(),
            2 => self.bind_address.as_str(),
            3 => self.port.as_str(),
            4 => self.active.as_str(),
            5 => self.errors.as_str(),
            6 => self.overall.as_str(),
            7 => self.age.as_deref().unwrap_or("n/a"),
            _ => "n/a",
        })
    }

    fn column_sort_text(&self, column: usize) -> &str {
        match column {
            0 => self.group(),
            1 => self.name(),
            2 => self.bind_address.as_str(),
            3 => self.port_sort.as_str(),
            4 => self.active_sort.as_str(),
            5 => self.errors_sort.as_str(),
            6 => self.overall_sort.as_str(),
            7 => self.age.as_deref().unwrap_or("n/a"),
            _ => "n/a",
        }
    }
}

impl From<&PortForwardItem> for ContainerRef {
    fn from(value: &PortForwardItem) -> Self {
        Self::simple(value.name.clone(), value.group.as_str().into(), value.container.clone())
    }
}

/// Filtering context for [`PortForwardItem`].
pub struct PortForwardFilterContext {
    pattern: Option<Expression>,
}

impl FilterContext for PortForwardFilterContext {
    fn restart(&mut self) {
        // Empty implementation.
    }
}

impl Filterable<PortForwardFilterContext> for PortForwardItem {
    fn get_context(pattern: &str, _settings: Option<&str>) -> PortForwardFilterContext {
        PortForwardFilterContext {
            pattern: parse(pattern).ok(),
        }
    }

    fn is_matching(&self, context: &mut PortForwardFilterContext) -> bool {
        if let Some(expression) = &context.pattern {
            self.filter_metadata.evaluate(expression)
        } else {
            false
        }
    }
}

fn get_filter_metadata(task: &PortForwardTask) -> SelectiveMap {
    let mut result = SelectiveMap::default()
        .with("n", vec![task.resource.name.as_deref().unwrap_or_default().to_owned()])
        .with_explicit("ns", vec![task.resource.namespace.as_str().to_owned()]);

    result.set_optional("l");
    result.set_optional("a");
    result
}
