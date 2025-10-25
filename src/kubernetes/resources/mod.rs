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

pub use self::crd_columns::*;
pub use self::data::*;
pub use self::ports::*;
pub use self::resource::*;
pub use self::resources_list::*;

mod crd_columns;
mod data;
mod ports;
mod resource;
mod resources_list;
