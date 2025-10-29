pub use self::core::{
    ALL_NAMESPACES, CONTAINERS, CORE_VERSION, CRDS, DAEMON_SETS, DEPLOYMENTS, EVENTS, JOBS, NAMESPACES, NODES, PODS,
    REPLICA_SETS, SECRETS, SERVICES, STATEFUL_SETS,
};
pub use self::core::{Kind, Namespace, PodRef, ResourceRef, ResourceRefFilter};

pub mod client;

mod core;
