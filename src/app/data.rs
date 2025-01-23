use kube::discovery::Scope;
use std::{cell::RefCell, rc::Rc};

use crate::kubernetes::Namespace;

use super::Config;

pub type SharedAppData = Rc<RefCell<AppData>>;

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
}

/// Contains all data that can be shared in the application.
#[derive(Default)]
pub struct AppData {
    /// Application configuration read from file.
    pub config: Config,

    /// Information about currently selected kubernetes resource.
    pub current: ResourcesInfo,

    /// Indicates if application is connected to the kubernetes api.
    pub is_connected: bool,
}

impl AppData {
    /// Creates new [`AppData`] instance.
    pub fn new(config: Config) -> Self {
        Self {
            config,
            current: ResourcesInfo::default(),
            is_connected: false,
        }
    }

    /// Returns resource's `kind` and `namespace` from the configuration.  
    /// **Note** that if provided `context` is not found in the configuration file, current context resource is used.
    pub fn get_namespaced_resource_from_config(&self, context: &str) -> (String, Namespace) {
        let kind = self.config.get_kind(context);
        if kind.is_none() {
            (self.current.kind_plural.clone(), self.current.namespace.clone())
        } else {
            let namespace = self.config.get_namespace(context).unwrap_or_default();
            (kind.unwrap_or_default().to_owned(), namespace.into())
        }
    }
}
