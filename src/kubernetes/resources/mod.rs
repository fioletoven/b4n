pub const CONTAINERS: &str = "containers";
pub const PODS: &str = "pods";

pub use self::data::*;
pub use self::kind::*;
pub use self::resource::*;

mod data;
mod kind;
mod resource;
