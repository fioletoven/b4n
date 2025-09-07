pub use self::kind::*;
pub use self::namespace::*;
pub use self::resource_ref::*;

pub mod client;
pub mod kinds;
pub mod resources;
pub mod utils;
pub mod watchers;

mod kind;
mod metrics;
mod namespace;
mod resource_ref;

/// Reference to the pod/container in a k8s cluster.
#[derive(Clone)]
pub struct PodRef {
    pub name: String,
    pub namespace: Namespace,
    pub container: Option<String>,
}
