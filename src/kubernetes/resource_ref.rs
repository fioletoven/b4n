use crate::kubernetes::{Kind, Namespace, resources::PODS};

/// Points to the specific kubernetes resource.\
/// **Note** that it can also point to the specific container or all containers in a pod.
#[derive(Default, Debug, Clone, PartialEq)]
pub struct ResourceRef {
    pub kind: Kind,
    pub namespace: Namespace,
    pub name: Option<String>,
    pub container: Option<String>,
    is_any_container: bool,
}

impl ResourceRef {
    /// Creates new [`ResourceRef`] for kubernetes resource expressed as `kind` and `namespace`.
    pub fn new(resource_kind: Kind, resource_namespace: Namespace) -> Self {
        Self {
            kind: resource_kind,
            namespace: resource_namespace,
            name: None,
            container: None,
            is_any_container: false,
        }
    }

    /// Creates new [`ResourceRef`] for kubernetes named resource expressed as `kind`, `namespace` and `name`.
    pub fn named(resource_kind: Kind, resource_namespace: Namespace, resource_name: String) -> Self {
        Self {
            kind: resource_kind,
            namespace: resource_namespace,
            name: Some(resource_name),
            container: None,
            is_any_container: false,
        }
    }

    /// Creates new [`ResourceRef`] for kubernetes pod container.
    pub fn container(pod_name: String, pod_namespace: Namespace, container_name: String) -> Self {
        Self {
            kind: PODS.into(),
            namespace: pod_namespace,
            name: Some(pod_name),
            container: Some(container_name),
            is_any_container: true,
        }
    }

    /// Creates new [`ResourceRef`] for kubernetes pod containers.
    pub fn containers(pod_name: String, pod_namespace: Namespace) -> Self {
        Self {
            kind: PODS.into(),
            namespace: pod_namespace,
            name: Some(pod_name),
            container: None,
            is_any_container: true,
        }
    }

    pub fn is_container(&self) -> bool {
        self.is_any_container
    }
}
