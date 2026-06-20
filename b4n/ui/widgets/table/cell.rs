use b4n_kube::stats::{CpuMetrics, MemoryMetrics};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::Time;
use k8s_openapi::jiff::Timestamp;
use k8s_openapi::serde_json::{Value, from_value};
use std::borrow::Cow;

/// Table cell.
#[derive(Default, Debug)]
pub struct Cell {
    text: Option<String>,
    sort_text: Option<String>,
    time: Option<Timestamp>,
    is_time: bool,
}

impl Cell {
    /// Creates new [`Cell`] instance from raw text.
    pub fn raw(text: &str, sort_text: &str) -> Self {
        Self {
            text: Some(text.to_string()),
            sort_text: Some(sort_text.to_string()),
            ..Default::default()
        }
    }

    /// Creates new [`Cell`] instance as a number value.
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

    /// Creates new [`Cell`] instance as an integer value.
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

    /// Creates new [`Cell`] instance as a time value.
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

    /// Sets cell raw text.
    pub fn set_raw_text(&mut self, text: String) {
        self.text = Some(text);
        self.sort_text = None;
    }

    /// Returns cell raw text.
    pub fn raw_text(&self) -> Option<&str> {
        self.text.as_deref()
    }

    /// Returns cell value that can be used for sorting.
    pub fn sort_text(&self) -> &str {
        if let Some(sort_text) = &self.sort_text {
            sort_text
        } else {
            self.text.as_deref().unwrap_or("n/a")
        }
    }

    /// Returns cell text.
    pub fn text(&self) -> Cow<'_, str> {
        if self.is_time {
            Cow::Owned(self.time.as_ref().map_or("n/a".to_owned(), b4n_kube::utils::format_datetime))
        } else {
            Cow::Borrowed(self.text.as_deref().unwrap_or("n/a"))
        }
    }
}

impl From<Option<String>> for Cell {
    fn from(value: Option<String>) -> Self {
        Cell {
            text: value,
            ..Default::default()
        }
    }
}

impl From<String> for Cell {
    fn from(value: String) -> Self {
        Cell {
            text: Some(value),
            ..Default::default()
        }
    }
}

impl From<Option<&str>> for Cell {
    fn from(value: Option<&str>) -> Self {
        Cell {
            text: value.map(String::from),
            ..Default::default()
        }
    }
}

impl From<&str> for Cell {
    fn from(value: &str) -> Self {
        Cell {
            text: Some(value.into()),
            ..Default::default()
        }
    }
}

impl From<bool> for Cell {
    fn from(value: bool) -> Self {
        Cell {
            text: Some(value.to_string()),
            ..Default::default()
        }
    }
}

impl From<Option<&Timestamp>> for Cell {
    fn from(value: Option<&Timestamp>) -> Self {
        Self {
            text: value.map(b4n_kube::utils::format_datetime),
            sort_text: value.map(|v| v.as_millisecond().to_string()),
            ..Default::default()
        }
    }
}

impl From<Option<CpuMetrics>> for Cell {
    fn from(value: Option<CpuMetrics>) -> Self {
        value.map(Into::into).unwrap_or_default()
    }
}

impl From<CpuMetrics> for Cell {
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

impl From<Option<MemoryMetrics>> for Cell {
    fn from(value: Option<MemoryMetrics>) -> Self {
        value.map(Into::into).unwrap_or_default()
    }
}

impl From<MemoryMetrics> for Cell {
    fn from(value: MemoryMetrics) -> Self {
        Self {
            text: Some(value.rounded()),
            sort_text: Some(format!("{:0>width$}", value.value, width = 25)),
            ..Default::default()
        }
    }
}
