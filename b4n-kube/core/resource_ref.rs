use k8s_openapi::jiff::Timestamp;
use kube::api::ApiResource;

use super::{Kind, Namespace, PODS};

/// Reference to the pods container in a k8s cluster.
#[derive(Clone)]
pub struct ContainerRef {
    pub name: String,
    pub namespace: Namespace,
    pub container: Option<String>,
    pub is_init: bool,
    pub finished_at: Option<Timestamp>,
}

impl ContainerRef {
    /// Creates new [`ContainerRef`] instance.\
    /// **Note** that it checks if container name starts with `i:` and removes this prefix.
    pub fn new(name: String, namespace: Namespace, container: Option<ResourceTag>) -> Self {
        match container {
            Some(ResourceTag::Container(container, is_init, finished_at)) => Self {
                name,
                namespace,
                container: Some(container),
                is_init,
                finished_at,
            },
            _ => Self {
                name,
                namespace,
                container: None,
                is_init: false,
                finished_at: None,
            },
        }
    }

    /// Creates new simple [`ContainerRef`] instance.
    pub fn simple(name: String, namespace: Namespace, container: Option<String>) -> Self {
        Self {
            name,
            namespace,
            container,
            is_init: false,
            finished_at: None,
        }
    }
}

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

    /// Returns `true` if [`ResourceRef`] points to a filtered resource.
    pub fn is_filtered(&self) -> bool {
        self.filter.is_some()
    }
}

impl From<&ApiResource> for ResourceRef {
    fn from(value: &ApiResource) -> Self {
        Self {
            kind: Kind::new(&value.plural, &value.group, &value.version),
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
    pub fields: Option<String>,
    pub labels: Option<String>,
}

impl ResourceRefFilter {
    /// Creates new [`ResourceRefFilter`] instance from `name` and involved object's `uid`.
    pub fn involved(name: String, uid: &str) -> Self {
        Self {
            name: Some(name),
            fields: Some(format!("involvedObject.uid={uid}")),
            labels: None,
        }
    }

    /// Creates new [`ResourceRefFilter`] instance for a given `name` and `node_name`.
    pub fn node(name: String, node_name: &str) -> Self {
        Self {
            name: Some(name),
            fields: Some(format!("spec.nodeName={node_name}")),
            labels: None,
        }
    }

    /// Creates new [`ResourceRefFilter`] instance for a given `name` and `job_name`.
    pub fn job(name: String, job_name: &str) -> Self {
        Self {
            name: Some(name),
            fields: None,
            labels: Some(format!("job-name={job_name}")),
        }
    }

    /// Creates new [`ResourceRefFilter`] instance for a given `name` and `labels`.
    pub fn labels(name: String, labels: String) -> Self {
        Self {
            name: Some(name),
            fields: None,
            labels: Some(labels),
        }
    }
}

/// Possible resource tags.
#[derive(Debug, Clone, PartialEq)]
pub enum ResourceTag {
    MatchLabels(String),
    Container(String, bool, Option<Timestamp>),
    CpuStatistics(String),
    MemoryStatistics(String),
}
