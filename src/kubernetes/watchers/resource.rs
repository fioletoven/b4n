use delegate::delegate;
use k8s_openapi::serde_json::Value;
use kube::{
    api::{ApiResource, DynamicObject},
    discovery::{ApiCapabilities, Scope},
};
use std::collections::VecDeque;
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    kubernetes::{
        Kind, Namespace, ResourceRef,
        client::KubernetesClient,
        resources::ResourceItem,
        watchers::{BgObserver, BgObserverError, ObserverResult},
    },
    ui::widgets::FooterMessage,
};

/// Background k8s resource observer that emits [`ResourceItem`]s.
pub struct ResourceObserver {
    observer: BgObserver,
    queue: VecDeque<Box<ObserverResult<ResourceItem>>>,
}

impl ResourceObserver {
    /// Creates new [`ResourceObserver`] instance.
    pub fn new(footer_tx: UnboundedSender<FooterMessage>) -> Self {
        Self {
            observer: BgObserver::new(footer_tx),
            queue: VecDeque::with_capacity(200),
        }
    }

    delegate! {
        to self.observer {
            pub fn start(
                &mut self,
                client: &KubernetesClient,
                resource: ResourceRef,
                discovery: Option<(ApiResource, ApiCapabilities)>,
            ) -> Result<Scope, BgObserverError>;

            pub fn restart(
                &mut self,
                client: &KubernetesClient,
                new_resource: ResourceRef,
                discovery: Option<(ApiResource, ApiCapabilities)>,
            ) -> Result<Scope, BgObserverError>;

            pub fn restart_new_kind(
                &mut self,
                client: &KubernetesClient,
                new_kind: Kind,
                new_namespace: Namespace,
                discovery: Option<(ApiResource, ApiCapabilities)>,
            ) -> Result<Scope, BgObserverError>;

            pub fn restart_new_namespace(
                &mut self,
                client: &KubernetesClient,
                new_namespace: Namespace,
                discovery: Option<(ApiResource, ApiCapabilities)>,
            ) -> Result<Scope, BgObserverError>;

            pub fn cancel(&mut self);
            pub fn stop(&mut self);
            pub fn get_resource_kind(&self) -> &Kind;
            pub fn is_container(&self) -> bool;
            pub fn has_error(&self) -> bool;
        }
    }

    /// Restarts [`ResourceObserver`] task to watch pod containers.
    pub fn restart_containers(
        &mut self,
        client: &KubernetesClient,
        pod_name: String,
        pod_namespace: Namespace,
        discovery: Option<(ApiResource, ApiCapabilities)>,
    ) -> Result<Scope, BgObserverError> {
        if !self.observer.resource.is_container() || self.observer.resource.name.as_ref().is_none_or(|n| n != &pod_name) {
            let resource = ResourceRef::containers(pod_name, pod_namespace);
            self.observer.start(client, resource, discovery)?;
        }

        Ok(self.observer.scope.clone())
    }

    /// Tries to get next [`ObserverResult`].
    pub fn try_next(&mut self) -> Option<Box<ObserverResult<ResourceItem>>> {
        if let Some(result) = self.queue.pop_front() {
            return Some(result);
        }

        if let Some(result) = self.observer.try_next() {
            match *result {
                ObserverResult::Init(init_data) => {
                    self.queue.clear();
                    Some(Box::new(ObserverResult::Init(init_data)))
                },
                ObserverResult::InitDone => Some(Box::new(ObserverResult::InitDone)),
                ObserverResult::Apply(item) => self.get_next_result(item, false),
                ObserverResult::Delete(item) => self.get_next_result(item, true),
            }
        } else {
            None
        }
    }

    /// Drains waiting [`ObserverResult`]s.
    pub fn drain(&mut self) {
        self.observer.drain();
        self.queue.clear();
    }

    fn get_next_result(&mut self, object: DynamicObject, is_delete: bool) -> Option<Box<ObserverResult<ResourceItem>>> {
        self.queue_results(object, is_delete);
        self.queue.pop_front()
    }

    fn queue_results(&mut self, object: DynamicObject, is_delete: bool) {
        if self.observer.is_container() {
            self.queue_containers(&object, "initContainers", "initContainerStatuses", true, is_delete);
            self.queue_containers(&object, "containers", "containerStatuses", false, is_delete);
        } else {
            self.queue_resource(object, is_delete);
        }
    }

    fn queue_containers(&mut self, object: &DynamicObject, array: &str, statuses_array: &str, is_init: bool, is_delete: bool) {
        if let Some(containers) = get_containers(object, array) {
            for c in containers {
                let result = get_container_result(c, object, statuses_array, is_init, is_delete);
                self.queue.push_back(Box::new(result));
            }
        }
    }

    fn queue_resource(&mut self, object: DynamicObject, is_delete: bool) {
        let kind = self.observer.init.as_ref().map(|i| i.kind.as_str()).unwrap_or("");
        let result = ObserverResult::new(ResourceItem::from(kind, object), is_delete);
        self.queue.push_back(Box::new(result));
    }
}

fn get_containers<'a>(object: &'a DynamicObject, array_name: &str) -> Option<&'a Vec<Value>> {
    object
        .data
        .get("spec")
        .and_then(|s| s.get(array_name))
        .and_then(|c| c.as_array())
}

fn get_container_result(
    container: &Value,
    object: &DynamicObject,
    statuses_array: &str,
    is_init_container: bool,
    is_delete: bool,
) -> ObserverResult<ResourceItem> {
    let status = object
        .data
        .get("status")
        .and_then(|s| s.get(statuses_array))
        .and_then(|s| s.as_array())
        .and_then(|s| s.iter().find(|s| s["name"].as_str() == container["name"].as_str()));

    ObserverResult::new(
        ResourceItem::from_container(container, status, &object.metadata, is_init_container),
        is_delete,
    )
}
