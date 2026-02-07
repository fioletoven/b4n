use b4n_kube::crds::CrdColumns;
use b4n_kube::stats::Statistics;
use b4n_tui::table::Header;
use kube::api::DynamicObject;

use crate::kube::resources::ResourceData;

pub mod api_service;
pub mod config_map;
pub mod container;
pub mod crd;
pub mod custom_resource;
pub mod daemon_set;
pub mod default;
pub mod deployment;
pub mod endpoint_slice;
pub mod endpoints;
pub mod event;
pub mod ingress;
pub mod job;
pub mod lease;
pub mod namespace;
pub mod network_policy;
pub mod node;
pub mod node_metrics;
pub mod persistent_volume;
pub mod persistent_volume_claim;
pub mod pod;
pub mod pod_metrics;
pub mod priority_class;
pub mod replica_set;
pub mod role_binding;
pub mod secret;
pub mod service;
pub mod service_account;
pub mod stateful_set;
pub mod storage_class;

/// Returns [`ResourceData`] for provided Kubernetes resource.
pub fn get_resource_data(
    kind: &str,
    group: &str,
    crd: Option<&CrdColumns>,
    stats: &Statistics,
    object: &DynamicObject,
    is_filtered: bool,
) -> ResourceData {
    if let Some(crd) = crd {
        return custom_resource::data(crd, object);
    }

    match (kind, group) {
        ("APIService", "apiregistration.k8s.io") => api_service::data(object),
        ("ConfigMap", "") => config_map::data(object),
        ("CustomResourceDefinition", "apiextensions.k8s.io") => crd::data(object),
        ("DaemonSet", "apps") => daemon_set::data(object),
        ("Deployment", "apps") => deployment::data(object),
        ("Endpoints", "") => endpoints::data(object),
        ("EndpointSlice", "discovery.k8s.io") => endpoint_slice::data(object),
        ("Event", "") => event::data(object, is_filtered),
        ("Ingress", "networking.k8s.io") => ingress::data(object),
        ("Job", "batch") => job::data(object),
        ("Lease", "coordination.k8s.io") => lease::data(object),
        ("Namespace", "") => namespace::data(object),
        ("NetworkPolicy", "networking.k8s.io") => network_policy::data(object),
        ("Node", "") => node::data(object, stats),
        ("NodeMetrics", "metrics.k8s.io") => node_metrics::data(object),
        ("PersistentVolume", "") => persistent_volume::data(object),
        ("PersistentVolumeClaim", "") => persistent_volume_claim::data(object),
        ("Pod", "") => pod::data(object, stats),
        ("PodMetrics", "metrics.k8s.io") => pod_metrics::data(object),
        ("PriorityClass", "scheduling.k8s.io") => priority_class::data(object),
        ("ReplicaSet", "apps") => replica_set::data(object),
        ("RoleBinding", "rbac.authorization.k8s.io") => role_binding::data(object),
        ("Secret", "") => secret::data(object),
        ("Service", "") => service::data(object),
        ("ServiceAccount", "") => service_account::data(object),
        ("StatefulSet", "apps") => stateful_set::data(object),
        ("StorageClass", "storage.k8s.io") => storage_class::data(object),

        _ => default::data(object),
    }
}

/// Returns [`Header`] for provided Kubernetes resource kind.
pub fn get_header_data(kind: &str, group: &str, crd: Option<&CrdColumns>, has_metrics: bool, is_filtered: bool) -> Header {
    if let Some(crd) = crd {
        return custom_resource::header(crd);
    }

    match (kind, group) {
        ("APIService", "apiregistration.k8s.io") => api_service::header(),
        ("ConfigMap", "") => config_map::header(),
        ("CustomResourceDefinition", "apiextensions.k8s.io") => crd::header(),
        ("DaemonSet", "apps") => daemon_set::header(),
        ("Deployment", "apps") => deployment::header(),
        ("Endpoints", "") => endpoints::header(),
        ("EndpointSlice", "discovery.k8s.io") => endpoint_slice::header(),
        ("Event", "") => event::header(is_filtered),
        ("Ingress", "networking.k8s.io") => ingress::header(),
        ("Job", "batch") => job::header(),
        ("Lease", "coordination.k8s.io") => lease::header(),
        ("Namespace", "") => namespace::header(),
        ("NetworkPolicy", "networking.k8s.io") => network_policy::header(),
        ("Node", "") => node::header(has_metrics),
        ("NodeMetrics", "metrics.k8s.io") => node_metrics::header(),
        ("PersistentVolume", "") => persistent_volume::header(),
        ("PersistentVolumeClaim", "") => persistent_volume_claim::header(),
        ("Pod", "") => pod::header(has_metrics),
        ("PodMetrics", "metrics.k8s.io") => pod_metrics::header(),
        ("PriorityClass", "scheduling.k8s.io") => priority_class::header(),
        ("ReplicaSet", "apps") => replica_set::header(),
        ("RoleBinding", "rbac.authorization.k8s.io") => role_binding::header(),
        ("Secret", "") => secret::header(),
        ("Service", "") => service::header(),
        ("ServiceAccount", "") => service_account::header(),
        ("StatefulSet", "apps") => stateful_set::header(),
        ("StorageClass", "storage.k8s.io") => storage_class::header(),

        ("Container", "") => container::header(has_metrics),
        _ => default::header(),
    }
}
