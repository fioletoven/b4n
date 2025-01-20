use std::fmt::Display;

pub const ALL_NAMESPACES: &str = "all";
pub const NAMESPACES: &str = "namespaces";

/// Represents kubernetes namespace.  
/// **Note** that it treats string `all` as a special case: all namespaces.
#[derive(Default, Clone, PartialEq)]
pub struct Namespace {
    value: Option<String>,
}

impl Display for Namespace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_all() {
            write!(f, "/ALL/")
        } else {
            write!(f, "'{}'", self.as_str())
        }
    }
}

impl Namespace {
    /// Creates new [`Namespace`] instance.
    pub fn from(value: String) -> Self {
        if value.is_empty() || value == ALL_NAMESPACES {
            Self { value: None }
        } else {
            Self { value: Some(value) }
        }
    }

    /// Creates new [`Namespace`] instance that represents all namespaces.
    pub fn all() -> Self {
        Self { value: None }
    }

    /// Extracts a string slice containing the entire [`Namespace`].
    #[inline]
    pub fn as_str(&self) -> &str {
        match &self.value {
            Some(value) => value.as_str(),
            None => ALL_NAMESPACES,
        }
    }

    /// Provides a [`Namespace`] as an option.
    #[inline]
    pub fn as_option(&self) -> Option<&str> {
        self.value.as_deref()
    }

    /// Returns `true` if the [`Namespace`] instance represents all namespaces.
    #[inline]
    pub const fn is_all(&self) -> bool {
        !self.value.is_some()
    }
}

impl From<Option<String>> for Namespace {
    fn from(value: Option<String>) -> Self {
        if value.as_deref().is_some_and(|v| v.is_empty() || v == ALL_NAMESPACES) {
            Self { value: None }
        } else {
            Self { value }
        }
    }
}

impl From<Option<&str>> for Namespace {
    fn from(value: Option<&str>) -> Self {
        if value.is_some_and(|v| v.is_empty() || v == ALL_NAMESPACES) {
            Self { value: None }
        } else {
            Self {
                value: value.map(String::from),
            }
        }
    }
}

impl From<String> for Namespace {
    fn from(value: String) -> Self {
        if value.is_empty() || value == ALL_NAMESPACES {
            Self { value: None }
        } else {
            Self { value: Some(value) }
        }
    }
}

impl From<&str> for Namespace {
    fn from(value: &str) -> Self {
        if value.is_empty() || value == ALL_NAMESPACES {
            Self { value: None }
        } else {
            Self {
                value: Some(value.to_owned()),
            }
        }
    }
}

impl From<Namespace> for String {
    fn from(value: Namespace) -> Self {
        match value.value {
            Some(value) => value,
            None => ALL_NAMESPACES.to_owned(),
        }
    }
}

impl From<Namespace> for Option<String> {
    fn from(value: Namespace) -> Self {
        value.value
    }
}
