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

pub use self::kind::{CORE_VERSION, Kind};
pub use self::namespace::{ALL_NAMESPACES, NAMESPACES, Namespace};
pub use self::ports::{Port, PortProtocol};
pub use self::resource_ref::{PodRef, ResourceRef, ResourceRefFilter};

mod kind;
mod namespace;
mod ports;
mod resource_ref;
