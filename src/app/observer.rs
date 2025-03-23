use futures::TryStreamExt;
use kube::{
    api::{ApiResource, DynamicObject},
    discovery::{ApiCapabilities, Scope},
    runtime::{
        WatchStreamExt,
        watcher::{self, Error, Event, watcher},
    },
};
use std::{
    pin::pin,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};
use thiserror;
use tokio::{
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;
use tracing::{error, warn};

use crate::{
    kubernetes::{Namespace, client::KubernetesClient, resources::Resource},
    ui::widgets::FooterMessage,
};

use super::utils::wait_for_task;

const WATCH_ERROR_TIMEOUT_SECS: u64 = 120;

/// Possible errors from [`BgObserver`].
#[derive(thiserror::Error, Debug)]
pub enum BgObserverError {
    /// Resource was not found in k8s cluster
    #[error("kubernetes resource not found")]
    ResourceNotFound,
}

/// Background observer result.
pub enum ObserverResult {
    Init(InitData),
    InitDone,
    Apply(Resource),
    Delete(Resource),
}

impl ObserverResult {
    /// Creates new [`ObserverResult`] for resource.
    pub fn new(resource: Resource, is_delete: bool) -> Self {
        if is_delete {
            Self::Delete(resource)
        } else {
            Self::Apply(resource)
        }
    }
}

/// Data that is returned when [`BgObserver`] starts watching resource.
#[derive(Debug, Clone)]
pub struct InitData {
    pub name: Option<String>,
    pub kind: String,
    pub kind_plural: String,
    pub group: String,
    pub scope: Scope,
}

impl Default for InitData {
    fn default() -> Self {
        Self {
            name: None,
            kind: String::new(),
            kind_plural: String::new(),
            group: String::new(),
            scope: Scope::Cluster,
        }
    }
}

impl InitData {
    /// Creates new initial data for [`ObserverResult`].
    fn new(rt: &ResourceRef, ar: &ApiResource, scope: Scope) -> Self {
        if rt.is_container {
            Self {
                name: rt.name.clone(),
                kind: "Container".to_owned(),
                kind_plural: "containers".to_owned(),
                group: ar.group.clone(),
                scope,
            }
        } else {
            Self {
                name: None,
                kind: rt.kind.clone(),
                kind_plural: ar.plural.to_lowercase(),
                group: ar.group.clone(),
                scope,
            }
        }
    }
}

/// Points to the specific kubernetes resource.
#[derive(Default, Debug, Clone)]
pub struct ResourceRef {
    pub name: Option<String>,
    pub kind: String,
    pub namespace: Namespace,
    pub is_container: bool,
}

impl ResourceRef {
    /// Creates new [`ResourceRef`] for kubernetes resource expressed as `kind` and `namespace`.
    pub fn new(resource_kind: String, resource_namespace: Namespace) -> Self {
        Self {
            name: None,
            kind: resource_kind,
            namespace: resource_namespace,
            is_container: false,
        }
    }

    /// Creates new [`ResourceRef`] for kubernetes pod containers.
    pub fn container(pod_name: String, pod_namespace: Namespace) -> Self {
        Self {
            name: Some(pod_name),
            kind: "Pod".to_owned(),
            namespace: pod_namespace,
            is_container: true,
        }
    }
}

/// Background k8s resource observer.
pub struct BgObserver {
    resource: ResourceRef,
    scope: Scope,
    task: Option<JoinHandle<()>>,
    cancellation_token: Option<CancellationToken>,
    context_tx: UnboundedSender<Box<ObserverResult>>,
    context_rx: UnboundedReceiver<Box<ObserverResult>>,
    footer_tx: UnboundedSender<FooterMessage>,
    has_error: Arc<AtomicBool>,
}

impl BgObserver {
    /// Creates new [`BgObserver`] instance.
    pub fn new(footer_tx: UnboundedSender<FooterMessage>) -> Self {
        let (context_tx, context_rx) = mpsc::unbounded_channel();
        Self {
            resource: ResourceRef::default(),
            scope: Scope::Cluster,
            task: None,
            cancellation_token: None,
            context_tx,
            context_rx,
            footer_tx,
            has_error: Arc::new(AtomicBool::new(true)),
        }
    }
}

impl BgObserver {
    /// Starts new [`BgObserver`] task.  
    /// **Note** that it stops the old task if it is running.
    pub fn start(
        &mut self,
        client: &KubernetesClient,
        resource: ResourceRef,
        discovery: Option<(ApiResource, ApiCapabilities)>,
    ) -> Result<Scope, BgObserverError> {
        self.stop();

        let cancellation_token = CancellationToken::new();
        let (ar, cap) = discovery.ok_or(BgObserverError::ResourceNotFound)?;

        self.resource = resource;
        self.scope = cap.scope.clone();
        self.has_error.store(false, Ordering::Relaxed);

        let mut _processor = EventsProcessor {
            init_data: InitData::new(&self.resource, &ar, cap.scope.clone()),
            is_container: self.resource.is_container,
            context_tx: self.context_tx.clone(),
            footer_tx: self.footer_tx.clone(),
            has_error: Arc::clone(&self.has_error),
            last_watch_error: None,
        };
        let _api_client = client.get_api(ar, cap, self.resource.namespace.as_option(), self.resource.namespace.is_all());
        let _cancellation_token = cancellation_token.clone();
        let _resource_name = self.resource.name.clone();

        let task = tokio::spawn(async move {
            while !_cancellation_token.is_cancelled() {
                let mut config = watcher::Config::default();
                if let Some(name) = _resource_name.as_ref() {
                    let fields = format!("metadata.name={name}");
                    config = config.fields(&fields);
                }
                let watch = watcher(_api_client.clone(), config).default_backoff();
                let mut watch = pin!(watch);

                while !_cancellation_token.is_cancelled() {
                    tokio::select! {
                        _ = _cancellation_token.cancelled() => (),
                        result = watch.try_next() => {
                            if !_processor.process_event(result) {
                                // we need to restart watcher, so go up one while loop
                                break;
                            }
                        },
                    }
                }
            }
        });

        self.cancellation_token = Some(cancellation_token);
        self.task = Some(task);

        Ok(self.scope.clone())
    }

    /// Restarts [`BgObserver`] task if `new_kind` or `new_namespace` is different than the current one.
    pub fn restart(
        &mut self,
        client: &KubernetesClient,
        new_resource: ResourceRef,
        discovery: Option<(ApiResource, ApiCapabilities)>,
    ) -> Result<Scope, BgObserverError> {
        if self.resource.kind != new_resource.kind || self.resource.namespace != new_resource.namespace {
            self.start(client, new_resource, discovery)?;
        }

        Ok(self.scope.clone())
    }

    /// Restarts [`BgObserver`] task if `new_kind` is different from the current one.  
    /// **Note** that it uses `new_namespace` if resource is namespaced.
    pub fn restart_new_kind(
        &mut self,
        client: &KubernetesClient,
        new_kind: String,
        new_namespace: Namespace,
        discovery: Option<(ApiResource, ApiCapabilities)>,
    ) -> Result<Scope, BgObserverError> {
        if self.resource.kind != new_kind {
            let resource = if discovery.as_ref().is_some_and(|(_, cap)| cap.scope == Scope::Namespaced) {
                ResourceRef::new(new_kind, new_namespace)
            } else {
                ResourceRef::new(new_kind, Namespace::all())
            };

            self.start(client, resource, discovery)?;
        }

        Ok(self.scope.clone())
    }

    /// Restarts [`BgObserver`] task if `new_namespace` is different than the current one.
    pub fn restart_new_namespace(
        &mut self,
        client: &KubernetesClient,
        new_namespace: Namespace,
        discovery: Option<(ApiResource, ApiCapabilities)>,
    ) -> Result<Scope, BgObserverError> {
        if self.resource.namespace != new_namespace {
            let resource = ResourceRef::new(self.resource.kind.clone(), new_namespace);
            self.start(client, resource, discovery)?;
        }

        Ok(self.scope.clone())
    }

    /// Restarts [`BgObserver`] task to show pod containers.
    pub fn restart_containers(
        &mut self,
        client: &KubernetesClient,
        pod_name: String,
        pod_namespace: Namespace,
        discovery: Option<(ApiResource, ApiCapabilities)>,
    ) -> Result<Scope, BgObserverError> {
        if !self.resource.is_container || self.resource.name.as_ref().is_none_or(|n| n != &pod_name) {
            let resource = ResourceRef::container(pod_name, pod_namespace);
            self.start(client, resource, discovery)?;
        }

        Ok(self.scope.clone())
    }

    /// Cancels [`BgObserver`] task.
    pub fn cancel(&mut self) {
        if let Some(cancellation_token) = self.cancellation_token.take() {
            cancellation_token.cancel();
            self.resource = ResourceRef::default();
            self.has_error.store(true, Ordering::Relaxed);
        }
    }

    /// Cancels [`BgObserver`] task and waits until it is finished.
    pub fn stop(&mut self) {
        self.cancel();
        wait_for_task(self.task.take(), "discovery");
        self.drain();
    }

    /// Tries to get next [`ObserverResult`].
    pub fn try_next(&mut self) -> Option<Box<ObserverResult>> {
        self.context_rx.try_recv().ok()
    }

    /// Drains waiting [`ObserverResult`]s.
    pub fn drain(&mut self) {
        while self.context_rx.try_recv().is_ok() {}
    }

    /// Returns currently observed resource kind.
    pub fn get_resource_kind(&self) -> &str {
        &self.resource.kind
    }

    /// Returns `true` if observer is not running or is in an error state.
    pub fn has_error(&self) -> bool {
        self.has_error.load(Ordering::Relaxed)
    }
}

impl Drop for BgObserver {
    fn drop(&mut self) {
        self.cancel();
    }
}

/// Internal watcher's events processor.
#[derive(Debug)]
struct EventsProcessor {
    init_data: InitData,
    is_container: bool,
    context_tx: UnboundedSender<Box<ObserverResult>>,
    footer_tx: UnboundedSender<FooterMessage>,
    has_error: Arc<AtomicBool>,
    last_watch_error: Option<Instant>,
}

impl EventsProcessor {
    /// Process event received from the kubernetes resource watcher.  
    /// Returns `true` if all was OK or `false` if the watcher needs to be restarted.
    pub fn process_event(&mut self, result: Result<Option<Event<DynamicObject>>, Error>) -> bool {
        match result {
            Ok(event) => {
                let mut reset_error = true;
                match event {
                    Some(Event::Init) => {
                        reset_error = false; // Init is also emitted after a forced restart of the watcher
                        self.send_init_result();
                    }
                    Some(Event::InitDone) => self.context_tx.send(Box::new(ObserverResult::InitDone)).unwrap(),
                    Some(Event::InitApply(o) | Event::Apply(o)) => self.send_results(o, false),
                    Some(Event::Delete(o)) => self.send_results(o, true),
                    _ => (),
                }

                if reset_error {
                    self.last_watch_error = None;
                    self.has_error.store(false, Ordering::Relaxed);
                }
            }
            Err(error) => {
                let msg = format!("Watch {}: {}", self.init_data.kind_plural, error);
                warn!("{}", msg);
                self.footer_tx.send(FooterMessage::error(msg, 0)).unwrap();

                match error {
                    Error::WatchStartFailed(_) | Error::WatchFailed(_) => {
                        // WatchStartFailed and WatchFailed do not trigger Init, so we do not set error immediately.
                        if self
                            .last_watch_error
                            .is_some_and(|t| t.elapsed().as_secs() <= WATCH_ERROR_TIMEOUT_SECS)
                        {
                            warn!("Forcefully restarting watcher for {}", self.init_data.kind_plural);
                            self.has_error.store(true, Ordering::Relaxed);
                            self.last_watch_error = Some(Instant::now());

                            return false;
                        } else {
                            self.last_watch_error = Some(Instant::now());
                        }
                    }
                    _ => self.has_error.store(true, Ordering::Relaxed),
                }
            }
        }

        true
    }

    fn send_init_result(&self) {
        self.context_tx
            .send(Box::new(ObserverResult::Init(self.init_data.clone())))
            .unwrap();
    }

    fn send_results(&self, object: DynamicObject, is_delete: bool) {
        if self.is_container {
            if let Some(containers) = object
                .data
                .get("spec")
                .and_then(|s| s.get("containers"))
                .and_then(|c| c.as_array())
            {
                for c in containers.iter() {
                    let result = ObserverResult::new(Resource::from_container(c, &object.metadata), is_delete);
                    self.context_tx.send(Box::new(result)).unwrap();
                }
            }
        } else {
            self.context_tx
                .send(Box::new(ObserverResult::new(
                    Resource::from(&self.init_data.kind, object),
                    is_delete,
                )))
                .unwrap();
        }
    }
}
