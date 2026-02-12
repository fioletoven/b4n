pub const NODES: &str = "nodes";
pub const PODS: &str = "pods";
pub const CONTAINERS: &str = "containers";
pub const SERVICES: &str = "services";
pub const JOBS: &str = "jobs";
pub const DEPLOYMENTS: &str = "deployments";
pub const REPLICA_SETS: &str = "replicasets";
pub const DAEMON_SETS: &str = "daemonsets";
pub const STATEFUL_SETS: &str = "statefulsets";
pub const SECRETS: &str = "secrets";
pub const EVENTS: &str = "events";
pub const CRDS: &str = "customresourcedefinitions";
pub const PVC: &str = "persistentvolumeclaims";
pub const PV: &str = "persistentvolumes";

pub use self::kind::{CORE_VERSION, Kind};
pub use self::namespace::{ALL_NAMESPACES, NAMESPACES, Namespace};
pub use self::ports::{Port, PortProtocol};
pub use self::propagation_policy::PropagationPolicy;
pub use self::resource_ref::{ContainerRef, ResourceRef, ResourceRefFilter, ResourceTag};

mod kind;
mod namespace;
mod ports;
mod propagation_policy;
mod resource_ref;

const KNOWN_API_GROUPS: [&str; 23] = [
    "admissionregistration.k8s.io",
    "apiextensions.k8s.io",
    "apiregistration.k8s.io",
    "apps",
    "authentication.k8s.io",
    "authorization.k8s.io",
    "autoscaling",
    "batch",
    "certificates.k8s.io",
    "coordination.k8s.io",
    "core",
    "discovery.k8s.io",
    "events.k8s.io",
    "flowcontrol.apiserver.k8s.io",
    "internal.apiserver.k8s.io",
    "networking.k8s.io",
    "node.k8s.io",
    "policy",
    "rbac.authorization.k8s.io",
    "resource.k8s.io",
    "scheduling.k8s.io",
    "storage.k8s.io",
    "storagemigration.k8s.io",
];

/// Returns `true` if the specified group is a well known Kubernetes API group.
pub fn is_builtin_api_group(group: &str) -> bool {
    group.is_empty() || KNOWN_API_GROUPS.contains(&group)
}
