use crate::is_builtin_api_group;

use super::{CONTAINERS, NAMESPACES};

#[cfg(test)]
#[path = "./kind.tests.rs"]
mod kind_tests;

pub const CORE_VERSION: &str = "v1";

/// Represents kubernetes kind together with its group.\
/// **Note** that it can be also used for plural names.
#[derive(Default, Debug, Clone)]
pub struct Kind {
    name: String,
    group: Option<usize>,
    version: Option<usize>,
}

impl Kind {
    /// Creates new [`Kind`] instance.
    pub fn new(kind: &str, group: &str, version: &str) -> Self {
        if group.is_empty() && (version.is_empty() || version == CORE_VERSION) {
            kind.into()
        } else if version.is_empty() {
            format!("{kind}.{group}").into()
        } else {
            format!("{kind}.{group}/{version}").into()
        }
    }

    /// Creates new [`Kind`] instance from `kind` and `api_version` string slices.
    pub fn from_api_version(kind: &str, api_version: &str) -> Self {
        if api_version.is_empty() || api_version == CORE_VERSION {
            kind.into()
        } else if !api_version.contains('/') {
            format!("{kind}./{api_version}").into()
        } else {
            format!("{kind}.{api_version}").into()
        }
    }

    /// Returns `true` if kind represents namespaces.
    pub fn is_namespaces(&self) -> bool {
        self.name == NAMESPACES
    }

    /// Returns `true` if kind represents containers.
    pub fn is_containers(&self) -> bool {
        self.name == CONTAINERS
    }

    /// Returns kind as string slice.
    pub fn as_str(&self) -> &str {
        &self.name
    }

    /// Returns kind's name.
    pub fn name(&self) -> &str {
        if let Some(group) = self.group {
            &self.name[..group]
        } else {
            &self.name
        }
    }

    /// Returns `true` if kind has group.
    pub fn has_group(&self) -> bool {
        self.group.is_some() && self.group.map(|g| g + 1) != self.version
    }

    /// Returns kind's group.
    pub fn group(&self) -> &str {
        if let Some(group) = self.group {
            let group = group + 1;
            if let Some(version) = self.version {
                if group < version { &self.name[group..version] } else { "" }
            } else {
                &self.name[group..]
            }
        } else {
            ""
        }
    }

    /// Returns kind's name and group.
    pub fn name_and_group(&self) -> &str {
        if let Some(version) = self.version {
            &self.name[..version]
        } else {
            &self.name
        }
    }

    /// Returns `true` if kind has version.
    pub fn has_version(&self) -> bool {
        self.version.is_some()
    }

    /// Returns kind's version.
    pub fn version(&self) -> &str {
        if let Some(version) = self.version {
            &self.name[version + 1..]
        } else {
            ""
        }
    }

    /// Returns kind's api version.
    pub fn api_version(&self) -> &str {
        if let Some(group) = self.group {
            &self.name[group + 1..]
        } else {
            CORE_VERSION
        }
    }

    /// Returns `true` if this kind has a well known API group.
    pub fn is_builtin(&self) -> bool {
        is_builtin_api_group(self.group())
    }
}

impl PartialEq for Kind {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl From<String> for Kind {
    fn from(mut value: String) -> Self {
        let group = value.find('.');
        let version = value.find('/');

        if let Some(group) = group
            && let Some(version) = version
            && group + 1 == version
            && &value[version + 1..] == CORE_VERSION
        {
            value.truncate(group);
            Self {
                name: value,
                group: None,
                version: None,
            }
        } else {
            Self {
                name: value,
                group,
                version,
            }
        }
    }
}

impl From<&str> for Kind {
    fn from(value: &str) -> Self {
        let group = value.find('.');
        let version = value.find('/');

        if let Some(group) = group
            && let Some(version) = version
            && group + 1 == version
            && &value[version + 1..] == CORE_VERSION
        {
            Self {
                name: value[..group].to_owned(),
                group: None,
                version: None,
            }
        } else {
            Self {
                name: value.to_owned(),
                group,
                version,
            }
        }
    }
}

impl From<Kind> for String {
    fn from(value: Kind) -> Self {
        value.name
    }
}
