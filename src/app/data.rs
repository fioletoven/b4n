use kube::discovery::Scope;
use std::{cell::RefCell, rc::Rc};

use super::Config;

pub type SharedAppData = Rc<RefCell<AppData>>;

/// Kubernetes resources data
pub struct ResourcesInfo {
    pub context: String,
    pub namespace: String,
    pub version: String,
    pub kind: String,
    pub kind_plural: String,
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
            scope: Scope::Cluster,
            count: Default::default(),
        }
    }
}

impl ResourcesInfo {
    /// Creates new [`KubernetesInfo`] instance from provided values
    pub fn from(context: String, namespace: String, version: String, scope: Scope) -> Self {
        Self {
            context,
            namespace,
            version,
            scope,
            ..Default::default()
        }
    }
}

/// Contains all data that can be shared in the application
pub struct AppData {
    /// Application configuration read from file
    pub config: Config,

    /// Information about currently selected kubernetes resource
    pub current: ResourcesInfo,

    /// Indicates if application is connected to the kubernetes api
    pub is_connected: bool,
}

impl AppData {
    /// Creates new [`AppData`] instance
    pub fn new(config: Config) -> Self {
        Self {
            config,
            current: ResourcesInfo::default(),
            is_connected: false,
        }
    }
}
