use kube::discovery::Scope;
use std::{cell::RefCell, rc::Rc};
use syntect::{dumps::from_uncompressed_data, parsing::SyntaxSet};

use crate::{
    kubernetes::{Kind, Namespace},
    ui::theme::Theme,
};

use super::{Config, History, InitData};

pub type SharedAppData = Rc<RefCell<AppData>>;

pub const SYNTAX_SET_DATA: &[u8] = include_bytes!("../../assets/syntaxes/syntaxes.packdump");

/// Kubernetes resources data.
pub struct ResourcesInfo {
    pub context: String,
    pub namespace: Namespace,
    pub version: String,
    pub name: Option<String>,
    pub kind: Kind,
    pub scope: Scope,
    pub count: usize,
    is_all_namespace: bool,
}

impl Default for ResourcesInfo {
    fn default() -> Self {
        Self {
            context: String::default(),
            namespace: Namespace::default(),
            version: String::default(),
            name: None,
            kind: Kind::default(),
            scope: Scope::Cluster,
            count: Default::default(),
            is_all_namespace: false,
        }
    }
}

impl ResourcesInfo {
    /// Creates new [`ResourcesInfo`] instance from provided values.
    pub fn from(context: String, namespace: Namespace, version: String, scope: Scope) -> Self {
        Self {
            context,
            is_all_namespace: namespace.is_all(),
            namespace,
            version,
            scope,
            ..Default::default()
        }
    }

    /// Updates [`ResourcesInfo`] with data from the [`InitData`].\
    /// **Note** that this update do not change the flag `is_all_namespace`.
    /// This results in remembering if the `all` namespace was set by user or by [`InitData`].
    pub fn update_from(&mut self, data: &InitData) {
        self.name.clone_from(&data.name);
        self.namespace = data.namespace.clone();
        self.kind = Kind::new(&data.kind_plural, &data.group);
        self.scope = data.scope.clone();
    }

    /// Returns `true` if specified `namespace` is equal to the currently held by [`ResourcesInfo`].\
    /// **Note** that it takes into account the flag for `all` namespace.
    pub fn is_all_namespace(&self) -> bool {
        if self.is_all_namespace {
            true
        } else {
            self.namespace.is_all()
        }
    }

    /// Returns `true` if specified `namespace` is equal to the currently held by [`ResourcesInfo`].\
    /// **Note** that it takes into account the flag for `all` namespace.
    pub fn is_namespace_equal(&self, namespace: &Namespace) -> bool {
        if self.is_all_namespace {
            namespace.is_all()
        } else {
            self.namespace == *namespace
        }
    }

    /// Sets new namespace.\
    /// **Note** that it takes into account the flag for `all` namespace.
    pub fn set_namespace(&mut self, namespace: Namespace) {
        self.is_all_namespace = namespace.is_all();
        self.namespace = namespace;
    }

    /// Gets namespace respecting the flag if it is an `all` namespace.
    pub fn get_namespace(&self) -> Namespace {
        if self.is_all_namespace {
            Namespace::all()
        } else {
            self.namespace.clone()
        }
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

    /// Returns resource's `kind` and `namespace` from the history data.\
    /// **Note** that if provided `context` is not found in the history file, current context resource is used.
    pub fn get_namespaced_resource_from_config(&self, context: &str) -> (Kind, Namespace) {
        if let Some(kind) = self.history.get_kind(context) {
            let namespace = self.history.get_namespace(context).unwrap_or_default();
            (kind.into(), namespace.into())
        } else {
            (self.current.kind.clone(), self.current.namespace.clone())
        }
    }

    /// Returns new [`SyntaxData`] instance.\
    /// **Note** that all elements are cloned/build every time you call this method.
    pub fn get_syntax_data(&self) -> SyntaxData {
        SyntaxData {
            syntax_set: self.syntax_set.clone(),
            yaml_theme: self.theme.build_syntect_yaml_theme(),
        }
    }
}
