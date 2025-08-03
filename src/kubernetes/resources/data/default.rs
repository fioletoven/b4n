use kube::api::DynamicObject;
use std::rc::Rc;

use crate::{
    kubernetes::resources::ResourceData,
    ui::lists::{Header, NAMESPACE},
};

/// Returns [`ResourceData`] for any kubernetes resource.
pub fn data(object: &DynamicObject) -> ResourceData {
    let is_terminating = object.metadata.deletion_timestamp.is_some();
    ResourceData::new(Box::default(), is_terminating)
}

/// Returns [`Header`] for default kubernetes resource.
pub fn header() -> Header {
    Header::from(NAMESPACE, None, Rc::new([' ', 'N', 'A']))
}
