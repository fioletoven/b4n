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

pub struct CrdObserver {
    pub is_ready: bool,
    observer: BgObserver,
}

impl CrdObserver {
    /// Creates new [`CrdObserver`] instance.
    pub fn new(footer_tx: UnboundedSender<FooterMessage>) -> Self {
        Self {
            is_ready: false,
            observer: BgObserver::new(footer_tx),
        }
    }

    pub fn start(
        &mut self,
        client: &KubernetesClient,
        discovery: Option<(ApiResource, ApiCapabilities)>,
    ) -> Result<Scope, BgObserverError> {
        let resource = ResourceRef::new(Kind::from(CRDS), Namespace::all());
        self.is_ready = false;
        self.observer.start(client, resource, discovery)
    }

    delegate! {
        to self.observer {
            pub fn cancel(&mut self);
            pub fn stop(&mut self);
            pub fn get_resource_kind(&self) -> &Kind;
            pub fn has_error(&self) -> bool;
        }
    }

    pub fn update_list(&mut self, list: &mut Vec<CrdColumns>) -> bool {
        let mut updated = false;
        while let Some(item) = self.observer.try_next() {
            updated = true;
            match *item {
                ObserverResult::Init(_) => {
                    self.is_ready = false;
                    list.clear();
                },
                ObserverResult::InitDone => self.is_ready = true,
                ObserverResult::Apply(item) => apply(list, item),
                ObserverResult::Delete(item) => delete(list, item),
            }
        }

        updated
    }
}

fn apply(list: &mut Vec<CrdColumns>, item: DynamicObject) {
    let item = CrdColumns::from(item);
    if let Some(position) = position(list, &item) {
        list[position] = item;
    } else {
        list.push(item);
    }
}

fn delete(list: &mut Vec<CrdColumns>, item: DynamicObject) {
    let item = CrdColumns::from(item);
    if let Some(position) = position(list, &item) {
        list.remove(position);
    }
}

fn position(list: &[CrdColumns], item: &CrdColumns) -> Option<usize> {
    list.iter()
        .position(|x| (x.uid.is_some() && x.uid == item.uid) || (x.uid.is_none() && x.name == item.name))
}
