pub const CONTAINERS: &str = "containers";
pub const PODS: &str = "pods";
pub const SECRETS: &str = "secrets";

pub use self::data::*;
pub use self::resource::*;
pub use self::resources_list::*;

mod data;
mod resource;
mod resources_list;
