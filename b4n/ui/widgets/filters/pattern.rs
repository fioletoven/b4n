use b4n_common::{add_padding, truncate};
use b4n_list::{BasicFilterContext, Filterable, Row};
use std::{
    borrow::Cow,
    time::{Duration, SystemTime},
};

/// Filter pattern item.
pub struct PatternItem {
    pub value: String,
    pub creation_time: SystemTime,
}

impl Default for PatternItem {
    fn default() -> Self {
        Self {
            value: String::default(),
            creation_time: SystemTime::now(),
        }
    }
}

impl std::fmt::Display for PatternItem {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}::{}",
            self.value,
            self.creation_time
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0)
        )
    }
}

impl From<&str> for PatternItem {
    fn from(value: &str) -> Self {
        let elements = value.splitn(2, "::").collect::<Vec<_>>();
        if elements.len() == 2 {
            Self {
                value: elements[0].to_string(),
                creation_time: SystemTime::UNIX_EPOCH + Duration::from_secs(elements[1].parse::<u64>().unwrap_or(0)),
            }
        } else {
            Self {
                value: value.to_owned(),
                creation_time: SystemTime::now(),
            }
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
        format!("{1:<0$} [{2}]", width, truncate(&self.value, width), description)
    }

    fn column_text(&self, column: usize) -> Cow<'_, str> {
        Cow::Borrowed(match column {
            0 => "n/a",
            1 => &self.value,
            _ => "n/a",
        })
    }

    fn column_sort_text(&self, column: usize) -> &str {
        match column {
            0 => "n/a",
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
