use delegate::delegate;
use k8s_openapi::serde_json::Value;
use kube::{
    api::{ApiResource, DynamicObject},
    discovery::{ApiCapabilities, Scope},
};
use std::collections::VecDeque;
use tokio::runtime::Handle;

use crate::{
    core::SharedCrdsList,
    kubernetes::{
        Kind, Namespace, ResourceRef,
        client::KubernetesClient,
        metrics::Metrics,
        resources::{CrdColumns, PODS, ResourceItem},
        watchers::{BgObserverError, InitData, ObserverResult, PodStats, SharedStatistics, Statistics, observer::BgObserver},
    },
    ui::widgets::FooterTx,
};

/// Background k8s resource observer that emits [`ResourceItem`]s.
pub struct ResourceObserver {
    observer: BgObserver,
    queue: VecDeque<Box<ObserverResult<ResourceItem>>>,
    group: String,
    crds: SharedCrdsList,
    crd: Option<CrdColumns>,
    statistics: SharedStatistics,
}

impl ResourceObserver {
    /// Creates new [`ResourceObserver`] instance.
    pub fn new(runtime: Handle, crds: SharedCrdsList, statistics: SharedStatistics, footer_tx: FooterTx) -> Self {
        Self {
            observer: BgObserver::new(runtime, footer_tx),
            queue: VecDeque::with_capacity(200),
            group: String::default(),
            crds,
            crd: None,
            statistics,
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

            pub fn cancel(&mut self);
            pub fn stop(&mut self);
            pub fn get_resource_kind(&self) -> &Kind;
            pub fn is_container(&self) -> bool;
            pub fn is_ready(&self) -> bool;
            pub fn has_error(&self) -> bool;
        }
    }

    /// Restarts [`ResourceObserver`] task if `new_kind` is different from the current one.\
    /// **Note** that it uses `new_namespace` if resource is namespaced.
    pub fn restart_new_kind(
        &mut self,
        client: &KubernetesClient,
        new_kind: Kind,
        new_namespace: Namespace,
        discovery: Option<(ApiResource, ApiCapabilities)>,
    ) -> Result<Scope, BgObserverError> {
        if self.observer.resource.kind != new_kind || self.observer.resource.is_container() != new_kind.is_containers() {
            let resource = if discovery.as_ref().is_some_and(|(_, cap)| cap.scope == Scope::Namespaced) {
                ResourceRef::new(new_kind, new_namespace)
            } else {
                ResourceRef::new(new_kind, Namespace::all())
            };

            self.restart(client, resource, discovery)?;
        }

        Ok(self.observer.scope.clone())
    }

    /// Restarts [`ResourceObserver`] task if `new_namespace` is different than the current one.
    pub fn restart_new_namespace(
        &mut self,
        client: &KubernetesClient,
        new_namespace: Namespace,
        discovery: Option<(ApiResource, ApiCapabilities)>,
    ) -> Result<Scope, BgObserverError> {
        if self.observer.resource.is_container() {
            let resource = ResourceRef::new(PODS.into(), new_namespace);
            self.restart(client, resource, discovery)?;
        } else if self.observer.resource.namespace != new_namespace {
            let resource = ResourceRef::new(self.observer.resource.kind.clone(), new_namespace);
            self.restart(client, resource, discovery)?;
        }

        Ok(self.observer.scope.clone())
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
            self.restart(client, resource, discovery)?;
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
                ObserverResult::Init(mut init_data) => {
                    self.queue.clear();
                    self.inject_init_data(&mut init_data);
                    self.group.clone_from(&init_data.group);
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
            let stats = &self.statistics.borrow();
            let pod_stats = get_pod_statistics(object, stats);
            for c in containers {
                let metrics = get_container_metrics(c, pod_stats, stats.has_metrics);
                let result = get_container_result(c, object, statuses_array, metrics, is_init, is_delete);
                self.queue.push_back(Box::new(result));
            }
        }
    }

    fn queue_resource(&mut self, object: DynamicObject, is_delete: bool) {
        let kind = self.observer.init.as_ref().map_or("", |i| i.kind.as_str());
        let result = ObserverResult::new(
            ResourceItem::from(
                kind,
                self.group.as_str(),
                self.crd.as_ref(),
                &self.statistics.borrow(),
                object,
            ),
            is_delete,
        );
        self.queue.push_back(Box::new(result));
    }

    /// Injects additional data to the [`InitData`] for observed resources.
    fn inject_init_data(&mut self, init_data: &mut InitData) {
        let kind = Kind::new(&init_data.kind_plural, &init_data.group);
        self.crd = self.crds.borrow().iter().find(|i| i.name == kind.as_str()).cloned();
        init_data.crd.clone_from(&self.crd);
        init_data.has_metrics = self.statistics.borrow().has_metrics;
    }
}

fn get_containers<'a>(object: &'a DynamicObject, array_name: &str) -> Option<&'a Vec<Value>> {
    object
        .data
        .get("spec")
        .and_then(|s| s.get(array_name))
        .and_then(|c| c.as_array())
}

fn get_pod_statistics<'a>(object: &DynamicObject, statistics: &'a Statistics) -> Option<&'a PodStats> {
    if statistics.has_metrics
        && let Some(node_name) = object.data["spec"]["nodeName"].as_str()
        && let Some(pod_name) = object.metadata.name.as_deref()
        && let Some(pod_namespace) = object.metadata.namespace.as_deref()
    {
        statistics.pod(node_name, pod_name, pod_namespace)
    } else {
        None
    }
}

fn get_container_metrics(container: &Value, pod_stats: Option<&PodStats>, has_metrics: bool) -> Option<Metrics> {
    let name = container["name"].as_str()?;
    let metrics = pod_stats.and_then(|pod| pod.container(name)).and_then(|c| c.metrics);

    match (has_metrics, metrics) {
        (false, Some(_)) => None,
        (true, None) => Some(Metrics::default()),
        _ => metrics,
    }
}

fn get_container_result(
    container: &Value,
    object: &DynamicObject,
    statuses_array: &str,
    metrics: Option<Metrics>,
    is_init_container: bool,
    is_delete: bool,
) -> ObserverResult<ResourceItem> {
    let status = object.data["status"][statuses_array]
        .as_array()
        .and_then(|s| s.iter().find(|s| s["name"].as_str() == container["name"].as_str()));

    ObserverResult::new(
        ResourceItem::from_container(container, status, &object.metadata, metrics, is_init_container),
        is_delete,
    )
}
