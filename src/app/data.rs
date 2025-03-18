use kube::discovery::Scope;
use std::{cell::RefCell, rc::Rc};
use syntect::{dumps::from_uncompressed_data, parsing::SyntaxSet};

use crate::{kubernetes::Namespace, ui::theme::Theme};

use super::{Config, History};

pub type SharedAppData = Rc<RefCell<AppData>>;

pub const SYNTAX_SET_DATA: &[u8] = include_bytes!("../../assets/syntaxes/syntaxes.packdump");

/// Kubernetes resources data.
pub struct ResourcesInfo {
    pub context: String,
    pub namespace: Namespace,
    pub version: String,
    pub kind: String,
    pub kind_plural: String,
    pub group: String,
    pub scope: Scope,
    pub count: usize,
}

impl Default for ResourcesInfo {
    fn default() -> Self {
        Self {
            context: Default::default(),
            namespace: Default::default(),
            version: Default::default(),
            kind: Default::default(),
            kind_plural: Default::default(),
            group: Default::default(),
            scope: Scope::Cluster,
            count: Default::default(),
        }
    }
}

impl ResourcesInfo {
    /// Creates new [`ResourcesInfo`] instance from provided values.
    pub fn from(context: String, namespace: Namespace, version: String, scope: Scope) -> Self {
        Self {
            context,
            namespace,
            version,
            scope,
            ..Default::default()
        }
    }

    /// Returns `true` if specified `kind` is equal to the currently held by [`ResourcesInfo`].
    pub fn is_kind_equal(&self, kind: &str) -> bool {
        if self.kind_plural == kind || self.kind.to_lowercase() == kind {
            return true;
        }

        kind.contains('.') && format!("{}.{}", self.kind_plural, self.group) == kind
    }
}

/// Keeps data required for syntax highlighting.
pub struct SyntaxData {
    pub syntax_set: SyntaxSet,
    pub yaml_theme: syntect::highlighting::Theme,
}

/// Contains all data that can be shared in the application.
#[derive(Default)]
pub struct AppData {
    /// Application configuration.
    pub config: Config,

    /// Application history data.
    pub history: History,

    /// Current application theme.
    pub theme: Theme,

    /// Information about currently selected kubernetes resource.
    pub current: ResourcesInfo,

    /// Syntax set for syntax highlighting.
    pub syntax_set: SyntaxSet,

    /// Indicates if application is connected to the kubernetes api.
    pub is_connected: bool,
}

impl AppData {
    /// Creates new [`AppData`] instance.
    pub fn new(config: Config, history: History, theme: Theme) -> Self {
        Self {
            config,
            history,
            theme,
            current: ResourcesInfo::default(),
            syntax_set: from_uncompressed_data::<SyntaxSet>(SYNTAX_SET_DATA).expect("cannot load SyntaxSet"),
            is_connected: false,
        }
    }

    /// Returns resource's `kind` and `namespace` from the history data.  
    /// **Note** that if provided `context` is not found in the history file, current context resource is used.
    pub fn get_namespaced_resource_from_config(&self, context: &str) -> (String, Namespace) {
        let kind = self.history.get_kind(context);
        if kind.is_none() {
            (self.current.kind_plural.clone(), self.current.namespace.clone())
        } else {
            let namespace = self.history.get_namespace(context).unwrap_or_default();
            (kind.unwrap_or_default().to_owned(), namespace.into())
        }
    }

    /// Returns new [`SyntaxData`] instance.  
    /// **Note** that all elements are cloned/build every time you call this method.
    pub fn get_syntax_data(&self) -> SyntaxData {
        SyntaxData {
            syntax_set: self.syntax_set.clone(),
            yaml_theme: self.theme.build_syntect_yaml_theme(),
        }
    }
}
