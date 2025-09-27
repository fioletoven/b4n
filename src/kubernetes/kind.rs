use crate::kubernetes::NAMESPACES;

use super::resources::CONTAINERS;

/// Represents kubernetes kind together with its group.\
/// **Note** that it can be also used for plural names.
#[derive(Default, Debug, Clone)]
pub struct Kind {
    kind: String,
    index: Option<usize>,
}

impl Kind {
    /// Creates new [`Kind`] instance.
    pub fn new(kind: &str, group: &str) -> Self {
        let kind = if group.is_empty() {
            kind.to_owned()
        } else {
            match group.split_once('/') {
                Some((group, _)) => format!("{kind}.{group}"),
                None => format!("{kind}.{group}"),
            }
        };
        let index = kind.find('.');
        Self { kind, index }
    }

    /// Creates new [`Kind`] instance from `kind` and `api_version` string slices.
    pub fn from_api_version(kind: &str, api_version: &str) -> Self {
        if api_version.contains('/') {
            Self::new(kind, api_version)
        } else {
            Self::new(kind, "")
        }
    }

    /// Returns `true` if kind represents namespaces.
    pub fn is_namespaces(&self) -> bool {
        self.kind == NAMESPACES
    }

    /// Returns `true` if kind represents containers.
    pub fn is_containers(&self) -> bool {
        self.kind == CONTAINERS
    }

    /// Returns `true` if kind has group.
    pub fn has_group(&self) -> bool {
        self.index.is_some()
    }

    /// Returns kind as string slice.
    pub fn as_str(&self) -> &str {
        &self.kind
    }

    /// Returns kind name.
    pub fn name(&self) -> &str {
        if let Some(index) = self.index {
            &self.kind[..index]
        } else {
            &self.kind
        }
    }

    /// Return kind group.
    pub fn group(&self) -> &str {
        if let Some(index) = self.index {
            &self.kind[index + 1..]
        } else {
            ""
        }
    }
}

impl PartialEq for Kind {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind
    }
}

impl From<String> for Kind {
    fn from(value: String) -> Self {
        let kind = match value.split_once('/') {
            Some((kind, _)) => kind.to_owned(),
            None => value,
        };

        let index = kind.find('.');
        Self { kind, index }
    }
}

impl From<&str> for Kind {
    fn from(value: &str) -> Self {
        let kind = match value.split_once('/') {
            Some((kind, _)) => kind.to_owned(),
            None => value.to_owned(),
        };

        let index = kind.find('.');
        Self { kind, index }
    }
}

impl From<Kind> for String {
    fn from(value: Kind) -> Self {
        value.kind
    }
}
