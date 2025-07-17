use delegate::delegate;
use kube::{
    api::{ApiResource, DynamicObject},
    discovery::{ApiCapabilities, Scope},
};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    kubernetes::{
        Kind, Namespace, ResourceRef,
        client::KubernetesClient,
        resources::{CRDS, CrdColumns},
        watchers::{BgObserverError, ObserverResult, observer::BgObserver},
    },
    ui::widgets::FooterMessage,
};

/// Custom resource definitions observer.
pub struct CrdObserver {
    observer: BgObserver,
}

impl CrdObserver {
    /// Creates new [`CrdObserver`] instance.
    pub fn new(footer_tx: UnboundedSender<FooterMessage>) -> Self {
        Self {
            observer: BgObserver::new(footer_tx),
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
        self.observer.start(client, resource, discovery)
    }

    delegate! {
        to self.observer {
            pub fn cancel(&mut self);
            pub fn stop(&mut self);
            pub fn get_resource_kind(&self) -> &Kind;
            pub fn is_ready(&self) -> bool;
            pub fn has_error(&self) -> bool;
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
    let item = CrdColumns::from(item);
    if let Some(position) = position(list, &item) {
        list[position] = item;
    } else {
        list.push(item);
    }
}

fn delete(list: &mut Vec<CrdColumns>, item: &DynamicObject) {
    let item = CrdColumns::from(item);
    if let Some(position) = position(list, &item) {
        list.remove(position);
    }
}

fn position(list: &[CrdColumns], item: &CrdColumns) -> Option<usize> {
    list.iter()
        .position(|x| (x.uid.is_some() && x.uid == item.uid) || (x.uid.is_none() && x.name == item.name))
}
