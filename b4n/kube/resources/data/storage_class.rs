use b4n_tui::table::{Column, Header, NAMESPACE};
use kube::api::DynamicObject;
use std::rc::Rc;

use crate::kube::resources::{ResourceData, ResourceValue};

/// Returns [`ResourceData`] for the `storageclass` kubernetes resource.
pub fn data(object: &DynamicObject) -> ResourceData {
    let is_terminating = object.metadata.deletion_timestamp.is_some();

    let values: [ResourceValue; 4] = [
        object.data["provisioner"].as_str().into(),
        object.data["reclaimPolicy"].as_str().into(),
        object.data["volumeBindingMode"].as_str().into(),
        object.data["allowVolumeExpansion"].as_str().into(),
    ];

    ResourceData::new(Box::new(values), is_terminating)
}

/// Returns [`Header`] for the `storageclass` kubernetes resource.
pub fn header() -> Header {
    Header::from(
        NAMESPACE,
        Some(Box::new([
            Column::bound("PROVISIONER", 12, 25, false),
            Column::fixed("RECLAIM POLICY", 14, false),
            Column::bound("VOLUME BINDING MODE", 18, 25, false),
            Column::fixed("ALLOW VOLUME EXPANSION", 20, false),
        ])),
        Rc::new([' ', 'N', 'P', 'R', 'V', 'L', 'A']),
    )
}
