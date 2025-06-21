use crate::{
    core::PortForwardTask,
    ui::{
        ViewType,
        colors::TextColors,
        lists::{BasicFilterContext, Filterable, Header, Row},
        theme::Theme,
    },
    utils::truncate,
};

/// Represents port forward list item.
pub struct PortForwardItem {
    pub uid: Option<String>,
    pub group: String,
    pub name: String,
}

impl PortForwardItem {
    /// Creates new [`PortForwardItem`] instance.
    pub fn from(task: &PortForwardTask) -> Self {
        Self {
            uid: Some(task.uuid.clone()),
            group: task.resource.namespace.as_str().to_owned(),
            name: task.resource.name.as_deref().unwrap_or_default().to_owned(),
        }
    }

    /// Returns [`TextColors`] for this port forward item considering `theme` and other data.
    pub fn get_colors(&self, theme: &Theme, is_active: bool, is_selected: bool) -> TextColors {
        theme.colors.line.ready.get_specific(is_active, is_selected)
    }

    /// Builds and returns the whole row of values for this port forward item.
    pub fn get_text(&self, view: ViewType, header: &Header, width: usize, namespace_width: usize, name_width: usize) -> String {
        format!("{} {}", self.group, self.name)
    }
}

impl Row for PortForwardItem {
    fn uid(&self) -> Option<&str> {
        self.uid.as_deref()
    }

    fn group(&self) -> &str {
        &self.group
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn get_name(&self, width: usize) -> String {
        format!("{1:<0$}", width, truncate(self.name.as_str(), width))
    }

    fn column_text(&self, column: usize) -> &str {
        match column {
            0 => &self.group,
            1 => self.name(),
            _ => "n/a",
        }
    }

    fn column_sort_text(&self, column: usize) -> &str {
        self.column_text(column)
    }
}

impl Filterable<BasicFilterContext> for PortForwardItem {
    fn get_context(pattern: &str, _: Option<&str>) -> BasicFilterContext {
        pattern.to_owned().into()
    }

    fn is_matching(&self, context: &mut BasicFilterContext) -> bool {
        self.name.contains(&context.pattern)
    }
}
