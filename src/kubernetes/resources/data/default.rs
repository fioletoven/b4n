use kube::api::DynamicObject;
use std::rc::Rc;

use crate::{
    kubernetes::resources::ResourceData,
    ui::lists::{Header, NAMESPACE},
};

/// Returns [`ResourceData`] for any kubernetes resource.
pub fn data(object: &DynamicObject) -> ResourceData {
    let is_terminating = object.metadata.deletion_timestamp.is_some();

    ResourceData {
        extra_values: Box::default(),
        is_job: false,
        is_completed: false,
        is_ready: !is_terminating,
        is_terminating,
    }
}

/// Returns [`Header`] for default kubernetes resource.
pub fn header() -> Header {
    Header::from(NAMESPACE, None, Rc::new([' ', 'N', 'A']))
}
