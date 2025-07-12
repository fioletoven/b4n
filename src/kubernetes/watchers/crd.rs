use delegate::delegate;
use kube::{
    ResourceExt,
    api::{ApiResource, DynamicObject},
    discovery::{ApiCapabilities, Scope},
};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    kubernetes::{
        Kind, Namespace, ResourceRef,
        client::KubernetesClient,
        watchers::{BgObserverError, ObserverResult, observer::BgObserver},
    },
    ui::widgets::FooterMessage,
};

pub const CRDS: &str = "customresourcedefinitions";

pub struct CrdPaths {
    pub uid: Option<String>,
    pub name: String,
}

impl CrdPaths {
    pub fn from(object: DynamicObject) -> Self {
        Self {
            uid: object.uid(),
            name: object.name_any(),
        }
    }
}

pub struct CrdObserver {
    observer: BgObserver,
    list: Vec<CrdPaths>,
}

impl CrdObserver {
    /// Creates new [`CrdObserver`] instance.
    pub fn new(footer_tx: UnboundedSender<FooterMessage>) -> Self {
        Self {
            observer: BgObserver::new(footer_tx),
            list: Vec::new(),
        }
    }

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
            pub fn has_error(&self) -> bool;
        }
    }

    pub fn list(&mut self) -> &[CrdPaths] {
        self.update_list();
        &self.list
    }

    pub fn update_list(&mut self) -> bool {
        let mut updated = false;
        while let Some(item) = self.observer.try_next() {
            updated = true;
            match *item {
                ObserverResult::Init(_) => self.list.clear(),
                ObserverResult::InitDone => (),
                ObserverResult::Apply(item) => self.apply(item),
                ObserverResult::Delete(item) => self.delete(item),
            }
        }

        updated
    }

    fn apply(&mut self, item: DynamicObject) {
        let item = CrdPaths::from(item);
        if let Some(position) = self.position(&item) {
            self.list[position] = item;
        } else {
            self.list.push(item);
        }
    }

    fn delete(&mut self, item: DynamicObject) {
        let item = CrdPaths::from(item);
        if let Some(position) = self.position(&item) {
            self.list.remove(position);
        }
    }

    fn position(&self, item: &CrdPaths) -> Option<usize> {
        self.list
            .iter()
            .position(|x| (x.uid.is_some() && x.uid == item.uid) || (x.uid.is_none() && x.name == item.name))
    }
}
