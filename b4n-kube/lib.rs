pub use self::core::{
    ALL_NAMESPACES, CONTAINERS, CORE_VERSION, CRDS, DAEMON_SETS, DEPLOYMENTS, EVENTS, JOBS, NAMESPACES, NODES, PODS,
    REPLICA_SETS, SECRETS, SERVICES, STATEFUL_SETS,
};
pub use self::core::{Kind, Namespace, PodRef, Port, PortProtocol, ResourceRef, ResourceRefFilter};
pub use self::discovery::{BgDiscovery, DiscoveryList, convert_to_vector};

pub mod client;

mod core;
mod discovery;
