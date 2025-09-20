use kube::api::ApiResource;

use crate::kubernetes::{Kind, Namespace, resources::PODS};

/// Points to the specific kubernetes resource.\
/// **Note** that it can also point to the specific container or all containers in a pod.
#[derive(Default, Debug, Clone, PartialEq)]
pub struct ResourceRef {
    pub kind: Kind,
    pub namespace: Namespace,
    pub name: Option<String>,
    pub filter: Option<ResourceRefFilter>,
    pub container: Option<String>,
    all_containers: bool,
}

impl ResourceRef {
    /// Creates new [`ResourceRef`] for a Kubernetes resource expressed as `kind` and `namespace`.
    pub fn new(resource_kind: Kind, resource_namespace: Namespace) -> Self {
        Self {
            kind: resource_kind,
            namespace: resource_namespace,
            name: None,
            filter: None,
            container: None,
            all_containers: false,
        }
    }

    /// Creates new [`ResourceRef`] for a Kubernetes resource that is narrowed down by the given `filter`.
    pub fn filtered(resource_kind: Kind, resource_namespace: Namespace, filter: ResourceRefFilter) -> Self {
        Self {
            kind: resource_kind,
            namespace: resource_namespace,
            name: None,
            filter: Some(filter),
            container: None,
            all_containers: false,
        }
    }

    /// Creates new [`ResourceRef`] for a Kubernetes named resource expressed as `kind`, `namespace` and `name`.
    pub fn named(resource_kind: Kind, resource_namespace: Namespace, resource_name: String) -> Self {
        Self {
            kind: resource_kind,
            namespace: resource_namespace,
            name: Some(resource_name),
            filter: None,
            container: None,
            all_containers: false,
        }
    }

    /// Creates new [`ResourceRef`] for a Kubernetes pod container.
    pub fn container(pod_name: String, pod_namespace: Namespace, container_name: String) -> Self {
        Self {
            kind: PODS.into(),
            namespace: pod_namespace,
            name: Some(pod_name),
            filter: None,
            container: Some(container_name),
            all_containers: false,
        }
    }

    /// Creates new [`ResourceRef`] for a Kubernetes pod containers.
    pub fn containers(pod_name: String, pod_namespace: Namespace) -> Self {
        Self {
            kind: PODS.into(),
            namespace: pod_namespace,
            name: Some(pod_name),
            filter: None,
            container: None,
            all_containers: true,
        }
    }

    /// Returns `true` if [`ResourceRef`] points to a specific container or containers.
    pub fn is_container(&self) -> bool {
        self.all_containers || self.container.is_some()
    }
}

impl From<&ApiResource> for ResourceRef {
    fn from(value: &ApiResource) -> Self {
        Self {
            kind: Kind::new(&value.plural, &value.group),
            namespace: Namespace::all(),
            name: None,
            filter: None,
            container: None,
            all_containers: false,
        }
    }
}

/// Optional filter for [`ResourceRef`] that can narrow down resources list.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct ResourceRefFilter {
    pub name: Option<String>,
    pub filter: Option<String>,
}

impl ResourceRefFilter {
    /// Creates new [`ResourceRefFilter`] instance from `name` and `uid`.
    pub fn new(name: String, uid: &str) -> Self {
        Self {
            name: Some(name),
            filter: Some(format!("involvedObject.uid={uid}")),
        }
    }
}
