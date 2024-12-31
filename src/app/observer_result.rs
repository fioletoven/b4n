use kube::{
    api::{DynamicObject, ObjectList},
    discovery::Scope,
};

/// Background observer result
pub struct ObserverResult {
    pub kind: String,
    pub kind_plural: String,
    pub scope: Scope,
    pub list: ObjectList<DynamicObject>,
}

impl ObserverResult {
    /// Creates new background observer result
    pub fn new(kind: String, kind_plural: String, scope: Scope, list: ObjectList<DynamicObject>) -> Self {
        ObserverResult {
            kind,
            kind_plural,
            scope,
            list,
        }
    }
}
