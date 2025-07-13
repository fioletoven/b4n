use kube::{ResourceExt, api::DynamicObject};

/// Holds data about custom columns defined in CRD resource.
#[derive(Debug, Clone)]
pub struct CrdColumns {
    pub uid: Option<String>,
    pub name: String,
}

impl CrdColumns {
    /// Creates new [`CrdColumns`] instance from [`DynamicObject`] resource.
    pub fn from(object: DynamicObject) -> Self {
        Self {
            uid: object.uid(),
            name: object.name_any(),
        }
    }
}
