use delegate::delegate;
use kube::ResourceExt;
use kube::api::{ApiResource, DynamicObject};
use kube::discovery::{ApiCapabilities, Scope};
use tokio::runtime::Handle;

use crate::client::KubernetesClient;
use crate::crds::CrdColumns;
use crate::{BgObserver, BgObserverError, CRDS, Kind, Namespace, ObserverResult, ResourceRef};

/// Custom resource definitions observer.
pub struct CrdObserver {
    observer: BgObserver,
}

impl CrdObserver {
    /// Creates new [`CrdObserver`] instance.
    pub fn new(runtime: Handle) -> Self {
        Self {
            observer: BgObserver::new(runtime, None),
        }
    }

    /// Starts new [`CrdObserver`] task.\
    /// **Note** that it stops the old task if it is running.
    pub fn start(
        &mut self,
        client: &KubernetesClient,
        discovery: Option<(ApiResource, ApiCapabilities)>,
    ) -> Result<Scope, BgObserverError> {
        let resource = ResourceRef::new(Kind::from(CRDS), Namespace::all());
        self.observer.start(client, resource, discovery, true)
    }

    delegate! {
        to self.observer {
            pub fn cancel(&mut self);
            pub fn stop(&mut self);
            pub fn observed_kind(&self) -> &Kind;
            pub fn is_ready(&self) -> bool;
            pub fn has_error(&self) -> bool;
            pub fn has_access(&self) -> bool;
        }
    }

    /// Updates provided [`CrdColumns`] list with waiting data.
    pub fn update_list(&mut self, list: &mut Vec<CrdColumns>) -> bool {
        let mut updated = false;
        while let Some(item) = self.observer.try_next() {
            updated = true;
            match *item {
                ObserverResult::Init(_) => list.clear(),
                ObserverResult::InitDone => (),
                ObserverResult::Apply(item) => apply(list, &item),
                ObserverResult::Delete(item) => delete(list, &item),
            }
        }

        updated
    }
}

fn apply(list: &mut Vec<CrdColumns>, item: &DynamicObject) {
    for item in get_for_all_versions(item) {
        if let Some(position) = list.iter().position(|x| x.uid == item.uid) {
            list[position] = item;
        } else {
            list.push(item);
        }
    }
}

fn delete(list: &mut Vec<CrdColumns>, item: &DynamicObject) {
    for item in get_for_all_versions(item) {
        if let Some(position) = list.iter().position(|x| x.uid == item.uid) {
            list.remove(position);
        }
    }
}

fn get_for_all_versions(item: &DynamicObject) -> Vec<CrdColumns> {
    let name = item.name_any();
    let uid = item.uid().unwrap_or_else(|| name.clone());
    item.data
        .get("spec")
        .and_then(|s| s.get("versions"))
        .and_then(|v| v.as_array())
        .map(|versions| versions.iter().map(|v| CrdColumns::from(&uid, &name, v)).collect())
        .unwrap_or_default()
}
