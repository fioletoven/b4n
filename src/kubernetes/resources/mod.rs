pub const CONTAINERS: &str = "containers";
pub const PODS: &str = "pods";
pub const NODES: &str = "nodes";
pub const JOBS: &str = "jobs";
pub const SERVICES: &str = "services";
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
