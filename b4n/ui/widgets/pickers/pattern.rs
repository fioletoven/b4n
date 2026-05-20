use b4n_common::truncate;
use b4n_config::HistoryItem;
use b4n_list::{BasicFilterContext, Filterable, Row};
use std::borrow::Cow;

/// Filter pattern item.
#[derive(Default)]
pub struct PatternItem {
    value: String,
    sort_value: Option<String>,
    icon: Option<&'static str>,
    is_fixed: bool,
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

    /// Returns pattern item value.
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Returns `true` if pattern item is fixed.
    pub fn is_fixed(&self) -> bool {
        self.is_fixed
    }

    /// Returns pattern item icon.
    pub fn icon(&self) -> Option<&str> {
        self.icon
    }

    /// Sets new icon value.
    pub fn set_icon(&mut self, icon: Option<&'static str>) {
        self.icon = icon;
    }

    /// Sets sort value.
    pub fn set_sort_value(&mut self, sort_value: Option<String>) {
        self.sort_value = sort_value;
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

impl From<&HistoryItem> for PatternItem {
    fn from(value: &HistoryItem) -> Self {
        PatternItem {
            value: value.value.clone(),
            ..Default::default()
        }
    }
}

impl From<PatternItem> for String {
    fn from(value: PatternItem) -> Self {
        value.value
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
            1 => match &self.sort_value {
                Some(sort_value) => sort_value,
                None => &self.value,
            },
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
