pub use self::kind::*;
pub use self::namespace::*;

pub mod client;
pub mod kinds;
pub mod resources;
pub mod utils;

mod kind;
mod namespace;

/// Reference to the pod/container in a k8s cluster.
pub struct PodRef {
    pub name: String,
    pub namespace: Namespace,
    pub container: Option<String>,
}
