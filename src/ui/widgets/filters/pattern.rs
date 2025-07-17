use std::{
    borrow::Cow,
    time::{Duration, SystemTime},
};

use crate::{
    ui::lists::{BasicFilterContext, Filterable, Row},
    utils::{add_padding, truncate},
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

impl From<String> for PatternItem {
    fn from(value: String) -> Self {
        if value.contains("::") {
            let mut split = value.splitn(2, "::");
            Self {
                value: split.next().map(String::from).unwrap(),
                creation_time: SystemTime::UNIX_EPOCH
                    + Duration::from_secs(split.next().map_or(0, |d| d.parse::<u64>().unwrap_or(0))),
            }
        } else {
            Self {
                value,
                creation_time: SystemTime::now(),
            }
        }
    }
}

impl Row for PatternItem {
    fn uid(&self) -> Option<&str> {
        Some(&self.value)
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

    fn get_name_for_highlighted(&self, width: usize) -> String {
        let width = width.saturating_sub(14);
        format!("{1:<0$} [TAB to insert]", width, truncate(&self.value, width))
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
