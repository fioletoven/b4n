use b4n_common::truncate;
use b4n_config::HistoryItem;
use b4n_list::{BasicFilterContext, Filterable, Row};
use std::borrow::Cow;
use std::time::SystemTime;

/// Filter pattern item.
pub struct PatternItem {
    pub value: String,
    pub creation_time: SystemTime,
    pub icon: Option<&'static str>,
    pub is_fixed: bool,
}

impl PatternItem {
    /// Creates new fixed [`PatternItem`] instance.
    pub fn fixed(value: String) -> Self {
        Self {
            value,
            is_fixed: true,
            ..Default::default()
        }
    }

    fn get_text_width(&self, width: usize) -> usize {
        self.icon
            .as_ref()
            .map_or(width, |i| width.saturating_sub(i.chars().count() + 1))
    }

    fn get_value_width(&self) -> usize {
        self.value.chars().filter(|c| *c != '␝').count()
    }

    fn add_icon(&self, text: &mut String) {
        if let Some(icon) = &self.icon {
            text.push(' ');
            text.push_str(icon);
        }
    }
}

impl Default for PatternItem {
    fn default() -> Self {
        Self {
            value: String::default(),
            creation_time: SystemTime::now(),
            icon: None,
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
        let text_width = self.get_text_width(width);
        let value_width = self.get_value_width();
        let padding_len = text_width.saturating_sub(value_width);

        let mut text = String::with_capacity(text_width + 2);
        text.push_str(truncate(&self.value, text_width));
        text.extend(std::iter::repeat_n(' ', padding_len));
        self.add_icon(&mut text);

        text
    }

    fn get_name_with_description(&self, width: usize, description: &str) -> String {
        let text_width = self.get_text_width(width);
        let value_width = self.get_value_width();
        let padding_len = text_width.saturating_sub(value_width);
        let description = truncate(description, padding_len.saturating_sub(1));

        let mut text = String::with_capacity(text_width + 2);
        text.push_str(truncate(&self.value, text_width));
        if description.is_empty() {
            text.extend(std::iter::repeat_n(' ', padding_len));
        } else {
            let padding_len = padding_len.saturating_sub(description.chars().count());
            text.extend(std::iter::repeat_n(' ', padding_len));
            text.push('␝');
            text.push_str(description);
            text.push('␝');
        }

        self.add_icon(&mut text);

        text
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
