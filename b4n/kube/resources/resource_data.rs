use b4n_config::themes::{TextColors, Theme};
use b4n_kube::stats::{CpuMetrics, MemoryMetrics};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::Time;
use k8s_openapi::jiff::Timestamp;
use k8s_openapi::serde_json::{Value, from_value};
use std::borrow::Cow;

/// Value for the resource extra data.
#[derive(Default)]
pub struct ResourceValue {
    text: Option<String>,
    sort_text: Option<String>,
    time: Option<Timestamp>,
    is_time: bool,
}

impl ResourceValue {
    /// Creates new [`ResourceValue`] instance as a number value.
    pub fn number(value: Option<f64>, len: u32) -> Self {
        let value = value.unwrap_or_default();
        let sort_value = value + (10u64.pow(len) as f64);
        Self {
            text: Some(format!("{:0.precision$}", value, precision = 3)),
            sort_text: Some(format!(
                "{:0>width$.precision$}",
                sort_value,
                width = (len as usize) + 5,
                precision = 3
            )),
            ..Default::default()
        }
    }

    /// Creates new [`ResourceValue`] instance as an integer value.
    pub fn integer(value: Option<i64>, len: u32) -> Self {
        let value = value.unwrap_or_default();
        let sort_value = value + 10i64.pow(len);
        let sort = format!("{:0>width$}", sort_value, width = (len as usize) + 1);
        Self {
            text: Some(value.to_string()),
            sort_text: Some(sort),
            ..Default::default()
        }
    }

    /// Creates new [`ResourceValue`] instance as a time value.
    pub fn time(value: Value) -> Self {
        let time = from_value::<Time>(value).ok().map(|t| t.0);
        let sort = time.as_ref().map(|t| t.as_millisecond().to_string());
        Self {
            time,
            sort_text: sort,
            is_time: true,
            ..Default::default()
        }
    }

    /// Returns resource raw text.
    pub fn raw_text(&self) -> Option<&str> {
        self.text.as_deref()
    }

    /// Returns resource value that can be used for sorting.
    pub fn sort_text(&self) -> &str {
        if let Some(sort_text) = &self.sort_text {
            sort_text
        } else {
            self.text.as_deref().unwrap_or("n/a")
        }
    }

    /// Returns resource text.
    pub fn text(&self) -> Cow<'_, str> {
        if self.is_time {
            Cow::Owned(self.time.as_ref().map_or("n/a".to_owned(), b4n_kube::utils::format_datetime))
        } else {
            Cow::Borrowed(self.text.as_deref().unwrap_or("n/a"))
        }
    }
}

impl From<Option<String>> for ResourceValue {
    fn from(value: Option<String>) -> Self {
        ResourceValue {
            text: value,
            ..Default::default()
        }
    }
}

impl From<String> for ResourceValue {
    fn from(value: String) -> Self {
        ResourceValue {
            text: Some(value),
            ..Default::default()
        }
    }
}

impl From<Option<&str>> for ResourceValue {
    fn from(value: Option<&str>) -> Self {
        ResourceValue {
            text: value.map(String::from),
            ..Default::default()
        }
    }
}

impl From<&str> for ResourceValue {
    fn from(value: &str) -> Self {
        ResourceValue {
            text: Some(value.into()),
            ..Default::default()
        }
    }
}

impl From<bool> for ResourceValue {
    fn from(value: bool) -> Self {
        ResourceValue {
            text: Some(value.to_string()),
            ..Default::default()
        }
    }
}

impl From<Option<&Timestamp>> for ResourceValue {
    fn from(value: Option<&Timestamp>) -> Self {
        Self {
            text: value.map(b4n_kube::utils::format_datetime),
            sort_text: value.map(|v| v.as_millisecond().to_string()),
            ..Default::default()
        }
    }
}

impl From<Option<CpuMetrics>> for ResourceValue {
    fn from(value: Option<CpuMetrics>) -> Self {
        value.map(Into::into).unwrap_or_default()
    }
}

impl From<CpuMetrics> for ResourceValue {
    fn from(value: CpuMetrics) -> Self {
        let text = value.millicores();
        let sort = format!("{:0>width$}", text, width = 10);
        Self {
            text: Some(text),
            sort_text: Some(sort),
            ..Default::default()
        }
    }
}

impl From<Option<MemoryMetrics>> for ResourceValue {
    fn from(value: Option<MemoryMetrics>) -> Self {
        value.map(Into::into).unwrap_or_default()
    }
}

impl From<MemoryMetrics> for ResourceValue {
    fn from(value: MemoryMetrics) -> Self {
        Self {
            text: Some(value.rounded()),
            sort_text: Some(format!("{:0>width$}", value.value, width = 25)),
            ..Default::default()
        }
    }
}

/// Extra data for the kubernetes resource.
#[derive(Default)]
pub struct ResourceData {
    pub is_completed: bool,
    pub is_ready: bool,
    pub is_terminating: bool,
    pub extra_values: Box<[ResourceValue]>,
    pub tags: Box<[String]>,
}

impl ResourceData {
    /// Creates new [`ResourceData`] instance.
    pub fn new(values: Box<[ResourceValue]>, is_terminating: bool) -> Self {
        Self {
            extra_values: values,
            is_ready: !is_terminating,
            is_terminating,
            ..Default::default()
        }
    }

    /// Adds tags to the [`ResourceData`] object.
    pub fn with_tags(mut self, tags: Box<[String]>) -> Self {
        self.tags = tags;
        self
    }

    /// Returns [`TextColors`] for the current resource state.
    pub fn get_colors(&self, theme: &Theme, is_active: bool, is_selected: bool) -> TextColors {
        if self.is_completed {
            theme.colors.line.completed.get_specific(is_active, is_selected)
        } else if self.is_terminating {
            theme.colors.line.terminating.get_specific(is_active, is_selected)
        } else if self.is_ready {
            theme.colors.line.ready.get_specific(is_active, is_selected)
        } else {
            theme.colors.line.in_progress.get_specific(is_active, is_selected)
        }
    }
}
