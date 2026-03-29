use b4n_common::{add_padding, truncate};
use b4n_config::HistoryItem;
use b4n_list::{BasicFilterContext, Filterable, Row};
use std::borrow::Cow;
use std::time::SystemTime;

/// Filter pattern item.
pub struct PatternItem {
    pub value: String,
    pub creation_time: SystemTime,
    pub description: Option<String>,
    pub is_fixed: bool,
}

impl PatternItem {
    /// Creates new fixed [`PatternItem`] instance.
    pub fn fixed(value: String, description: Option<String>) -> Self {
        Self {
            value,
            description,
            is_fixed: true,
            ..Default::default()
        }
    }
}

impl Default for PatternItem {
    fn default() -> Self {
        Self {
            value: String::default(),
            creation_time: SystemTime::now(),
            description: None,
            is_fixed: false,
        }
    }
}

impl From<&HistoryItem> for PatternItem {
    fn from(value: &HistoryItem) -> Self {
        PatternItem {
            value: value.value.clone(),
            creation_time: value.creation_time,
            ..Default::default()
        }
    }
}

impl Row for PatternItem {
    fn uid(&self) -> &str {
        &self.value
    }

    fn group(&self) -> &str {
        "n/a"
    }

    fn name(&self) -> &str {
        &self.value
    }

    fn get_name(&self, width: usize) -> String {
        add_padding(&self.value, width)
    }

    fn get_name_with_description(&self, width: usize, description: &str) -> String {
        let description = truncate(description, width.saturating_sub(1));
        let width = width.saturating_sub(description.len().saturating_add(1));
        format!("{1:<0$} ␝{2}␝", width, truncate(&self.value, width), description)
    }

    fn column_text(&self, column: usize) -> Cow<'_, str> {
        Cow::Borrowed(match column {
            1 => &self.value,
            _ => "n/a",
        })
    }

    fn column_sort_text(&self, column: usize) -> &str {
        match column {
            1 => &self.value,
            _ => "n/a",
        }
    }
}

impl Filterable<BasicFilterContext> for PatternItem {
    fn get_context(pattern: &str, _: Option<&str>) -> BasicFilterContext {
        pattern.to_owned().into()
    }

    fn is_matching(&self, context: &mut BasicFilterContext) -> bool {
        self.contains(&context.pattern)
    }
}
