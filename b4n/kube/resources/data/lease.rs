use b4n_tui::table::{Column, Header, NAMESPACE};
use kube::api::DynamicObject;
use std::rc::Rc;

use crate::kube::resources::{ResourceData, ResourceValue};

/// Returns [`ResourceData`] for the `lease` kubernetes resource.
pub fn data(object: &DynamicObject) -> ResourceData {
    let holder = object.data["spec"]["holderIdentity"].as_str();
    let is_terminating = object.metadata.deletion_timestamp.is_some();

    let values: [ResourceValue; 1] = [holder.into()];

    ResourceData::new(Box::new(values), is_terminating)
}

/// Returns [`Header`] for the `lease` kubernetes resource.
pub fn header() -> Header {
    Header::from(
        NAMESPACE,
        Some(Box::new([Column::bound("HOLDER", 12, 75, false)])),
        Rc::new([' ', 'N', 'H', 'A']),
    )
}
