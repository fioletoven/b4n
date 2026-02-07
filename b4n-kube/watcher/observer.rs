use b4n_common::NotificationSink;
use kube::Client;
use kube::api::{ApiResource, DynamicObject};
use kube::discovery::{ApiCapabilities, Scope, verbs};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use tokio::runtime::Handle;
use tokio::sync::mpsc::{self};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::watcher::client::{FallbackNamespace, ResourceClient};
use crate::watcher::list::{ListInput, list};
use crate::watcher::result::{ObserverResultReceiver, ObserverResultSender};
use crate::watcher::state::BgObserverHealth;
use crate::watcher::watch::{WatchInput, watch};
use crate::{BgObserverState, InitData, Kind, Namespace, ObserverResult, ResourceRef};

/// Possible errors from [`BgObserver`].
#[derive(thiserror::Error, Debug)]
pub enum BgObserverError {
    /// Resource was not found in k8s cluster
    #[error("kubernetes resource not found")]
    ResourceNotFound,

    /// Resource cannot be watched or listed
    #[error("resource cannot be watched or listed")]
    UnsupportedOperation,

    /// Observer is already started
    #[error("observer is already started")]
    AlreadyStarted,
}

/// Background k8s resource observer.
pub struct BgObserver {
    pub resource: ResourceRef,
    kind_singular: Option<String>,
    scope: Scope,
    runtime: Handle,
    task: Option<JoinHandle<()>>,
    cancellation_token: Option<CancellationToken>,
    context_tx: ObserverResultSender,
    context_rx: ObserverResultReceiver,
    footer_tx: Option<NotificationSink>,
    fallback: Option<Arc<Mutex<FallbackNamespace>>>,
    stop_on_access_error: bool,
    state: Arc<AtomicU8>,
    health: Arc<AtomicU8>,
    has_access: Arc<AtomicBool>,
    had_connection_error: Arc<AtomicBool>,
}

impl BgObserver {
    /// Creates new [`BgObserver`] instance.
    pub fn new(runtime: Handle, footer_tx: Option<NotificationSink>) -> Self {
        let (context_tx, context_rx) = mpsc::unbounded_channel();
        Self {
            resource: ResourceRef::default(),
            kind_singular: None,
            scope: Scope::Cluster,
            runtime,
            task: None,
            cancellation_token: None,
            context_tx,
            context_rx,
            footer_tx,
            fallback: None,
            stop_on_access_error: false,
            state: Arc::new(AtomicU8::new(BgObserverState::Idle.into())),
            health: Arc::new(AtomicU8::new(BgObserverHealth::Good.into())),
            has_access: Arc::new(AtomicBool::new(true)),
            had_connection_error: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Starts new [`BgObserver`] task.
    pub fn start(
        &mut self,
        client: Client,
        resource: ResourceRef,
        discovery: Option<(ApiResource, ApiCapabilities)>,
        fallback_namespace: Option<Namespace>,
        stop_on_access_error: bool,
    ) -> Result<Scope, BgObserverError> {
        if self.cancellation_token.is_some() {
            return Err(BgObserverError::AlreadyStarted);
        }

        self.state.store(BgObserverState::Connecting.into(), Ordering::Relaxed);
        self.health.store(BgObserverHealth::Good.into(), Ordering::Relaxed);
        self.had_connection_error.store(false, Ordering::Relaxed);

        let result = self.start_internal(client, resource, discovery, fallback_namespace, stop_on_access_error);
        if result.is_err() {
            self.state.store(BgObserverState::Idle.into(), Ordering::Relaxed);
        }

        result
    }

    /// Restarts [`BgObserver`] task if `new_resource` is different from the current one.\
    /// **Note** that it stops the old task if it is running.
    pub fn restart(
        &mut self,
        client: Client,
        new_resource: ResourceRef,
        discovery: Option<(ApiResource, ApiCapabilities)>,
        fallback_namespace: Option<Namespace>,
        stop_on_access_error: bool,
    ) -> Result<Scope, BgObserverError> {
        if self.resource == new_resource {
            return Ok(self.scope.clone());
        }

        let state: BgObserverState = self.state.load(Ordering::Relaxed).into();
        let health: BgObserverHealth = self.health.load(Ordering::Relaxed).into();
        let had_connection_error = health == BgObserverHealth::ConnectionError
            || (state == BgObserverState::Reconnecting && self.had_connection_error.load(Ordering::Relaxed));

        self.stop_internal();

        self.state.store(BgObserverState::Reconnecting.into(), Ordering::Relaxed);
        self.health.store(BgObserverHealth::Good.into(), Ordering::Relaxed);
        self.had_connection_error.store(had_connection_error, Ordering::Relaxed);

        let result = self.start_internal(client, new_resource, discovery, fallback_namespace, stop_on_access_error);
        if result.is_err() {
            self.state.store(BgObserverState::Idle.into(), Ordering::Relaxed);
        }

        result
    }

    /// Cancels [`BgObserver`] task.
    pub fn cancel(&mut self) {
        self.cancel_internal();
        self.state.store(BgObserverState::Idle.into(), Ordering::Relaxed);
    }

    /// Cancels [`BgObserver`] task and waits until it is finished.
    pub fn stop(&mut self) {
        self.stop_internal();
        self.state.store(BgObserverState::Idle.into(), Ordering::Relaxed);
    }

    /// Tries to get next [`ObserverResult`].
    pub fn try_next(&mut self) -> Option<Box<ObserverResult<DynamicObject>>> {
        self.context_rx.try_recv().ok()
    }

    /// Drains waiting [`ObserverResult`]s.
    pub fn drain(&mut self) {
        while self.context_rx.try_recv().is_ok() {}
    }

    /// Returns currently observed resource's kind.
    pub fn observed_kind(&self) -> &Kind {
        &self.resource.kind
    }

    /// Returns singular `PascalCase` name of the currently observed resource.
    pub fn observed_singular_kind(&self) -> Option<&str> {
        self.kind_singular.as_deref()
    }

    /// Returns initial resource's namespace.\
    /// **Note** that namespace can be outdated if the fallback namespace is used.
    pub fn initial_namespace(&self) -> &Namespace {
        &self.resource.namespace
    }

    /// Returns currently observed resource's scope.
    pub fn observed_resource_scope(&self) -> &Scope {
        &self.scope
    }

    /// Tries to change fallback namespace. It success only if the namespace is not already in use
    /// and fallback was set during the observer startup.
    pub fn try_change_fallback_namespace(&mut self, new_namespace: &Namespace) -> bool {
        if let Some(fallback) = self.fallback.as_ref()
            && let Ok(mut fallback) = fallback.lock()
        {
            if fallback.namespace == *new_namespace {
                return true;
            }

            if fallback.is_used {
                false
            } else {
                fallback.namespace = new_namespace.clone();
                true
            }
        } else {
            false
        }
    }

    /// Returns `true` if observer is running.
    pub fn is_running(&self) -> bool {
        self.task.is_some()
    }

    /// Returns `true` if the observed resource is a container.
    pub fn is_container(&self) -> bool {
        self.resource.is_container()
    }

    /// Returns `true` if the observed resource is filtered.
    pub fn is_filtered(&self) -> bool {
        self.resource.is_filtered()
    }

    /// Returns `true` if observer is in the connecting state.
    pub fn is_connecting(&self) -> bool {
        let health: BgObserverHealth = self.health.load(Ordering::Relaxed).into();
        let state: BgObserverState = self.state.load(Ordering::Relaxed).into();
        state == BgObserverState::Connecting && health != BgObserverHealth::ConnectionError
    }

    /// Returns `true` if observer is connected to the Kubernetes API or was connected and now is reconnecting.\
    /// **Note** that reconnecting almost always means observed kind switching.
    pub fn is_connected(&self) -> bool {
        let health: BgObserverHealth = self.health.load(Ordering::Relaxed).into();
        if health == BgObserverHealth::ConnectionError {
            return false;
        }

        let state: BgObserverState = self.state.load(Ordering::Relaxed).into();
        state == BgObserverState::Connected
            || state == BgObserverState::Ready
            || (state == BgObserverState::Reconnecting && !self.had_connection_error.load(Ordering::Relaxed))
    }

    /// Returns `true` if observer has received the initial list of resources.
    pub fn is_ready(&self) -> bool {
        BgObserverState::from(self.state.load(Ordering::Relaxed)) == BgObserverState::Ready
    }

    /// Returns `true` if observer is waiting for the Kubernetes API response.
    pub fn is_waiting(&self) -> bool {
        let health: BgObserverHealth = self.health.load(Ordering::Relaxed).into();
        if health != BgObserverHealth::Good {
            return false;
        }

        let state: BgObserverState = self.state.load(Ordering::Relaxed).into();
        state == BgObserverState::Connecting || state == BgObserverState::Reconnecting || state == BgObserverState::Connected
    }

    /// Returns `true` if user has access to the observed resource.
    pub fn has_access(&self) -> bool {
        self.has_access.load(Ordering::Relaxed)
    }

    /// Returns `true` if observer is in an error state.
    pub fn has_error(&self) -> bool {
        BgObserverHealth::from(self.health.load(Ordering::Relaxed)) != BgObserverHealth::Good
            || (BgObserverState::from(self.state.load(Ordering::Relaxed)) == BgObserverState::Reconnecting
                && self.had_connection_error.load(Ordering::Relaxed))
    }

    /// Returns `true` if observer is connected, but cannot use the Kubernetes API, e.g. it returns an error.
    pub fn has_api_error(&self) -> bool {
        BgObserverHealth::from(self.health.load(Ordering::Relaxed)) == BgObserverHealth::ApiError
    }

    fn start_internal(
        &mut self,
        client: Client,
        resource: ResourceRef,
        discovery: Option<(ApiResource, ApiCapabilities)>,
        fallback_namespace: Option<Namespace>,
        stop_on_access_error: bool,
    ) -> Result<Scope, BgObserverError> {
        let cancellation_token = CancellationToken::new();
        let (ar, cap) = discovery.ok_or(BgObserverError::ResourceNotFound)?;

        self.resource = resource;
        self.scope = cap.scope.clone();
        self.stop_on_access_error = stop_on_access_error;
        self.has_access.store(true, Ordering::Relaxed);
        if let Some(namespace) = fallback_namespace {
            self.fallback = Some(Arc::new(Mutex::new(FallbackNamespace {
                is_used: false,
                namespace,
            })));
        }

        let init_data = InitData::new(&self.resource, &ar, &cap, None, false);
        self.kind_singular = Some(init_data.kind.clone());

        let api_client = ResourceClient::new(client, ar, cap.clone(), self.resource.namespace.clone());
        let task = if cap.supports_operation(verbs::WATCH) {
            self.spawn_watch_task(api_client, init_data, cancellation_token.clone())
        } else if cap.supports_operation(verbs::LIST) {
            self.spawn_list_task(api_client, init_data, cancellation_token.clone())
        } else {
            return Err(BgObserverError::UnsupportedOperation);
        };

        self.cancellation_token = Some(cancellation_token);
        self.task = Some(task);

        Ok(self.scope.clone())
    }

    fn cancel_internal(&mut self) {
        if let Some(cancellation_token) = self.cancellation_token.take() {
            cancellation_token.cancel();
            self.resource = ResourceRef::default();
            self.has_access.store(true, Ordering::Relaxed);
        }
    }

    fn stop_internal(&mut self) {
        self.cancel_internal();
        b4n_common::tasks::wait_for_task(self.task.take(), "background observer");
        self.drain();
    }

    fn spawn_watch_task(
        &mut self,
        client: ResourceClient,
        init_data: InitData,
        cancellation_token: CancellationToken,
    ) -> JoinHandle<()> {
        self.runtime.spawn({
            watch(
                client,
                WatchInput {
                    init_data,
                    context_tx: self.context_tx.clone(),
                    footer_tx: self.footer_tx.clone(),
                    fallback: self.fallback.clone(),
                    state: Arc::clone(&self.state),
                    health: Arc::clone(&self.health),
                    has_access: Arc::clone(&self.has_access),
                },
                build_fields_filter(&self.resource),
                build_labels_filter(&self.resource),
                self.stop_on_access_error,
                cancellation_token,
            )
        })
    }

    fn spawn_list_task(
        &mut self,
        client: ResourceClient,
        init_data: InitData,
        cancellation_token: CancellationToken,
    ) -> JoinHandle<()> {
        self.runtime.spawn({
            list(
                client,
                ListInput {
                    init_data,
                    context_tx: self.context_tx.clone(),
                    footer_tx: self.footer_tx.clone(),
                    fallback: self.fallback.clone(),
                    state: Arc::clone(&self.state),
                    health: Arc::clone(&self.health),
                    has_access: Arc::clone(&self.has_access),
                },
                build_fields_filter(&self.resource),
                build_labels_filter(&self.resource),
                self.stop_on_access_error,
                cancellation_token,
            )
        })
    }
}

impl Drop for BgObserver {
    fn drop(&mut self) {
        self.cancel();
    }
}

fn build_fields_filter(rt: &ResourceRef) -> Option<String> {
    match (&rt.name, &rt.filter) {
        (Some(name), Some(filter)) => match &filter.fields {
            Some(data) => Some(format!("metadata.name={name},{data}")),
            None => Some(format!("metadata.name={name}")),
        },
        (Some(name), None) => Some(format!("metadata.name={name}")),
        (None, Some(filter)) => filter.fields.clone(),

        _ => None,
    }
}

fn build_labels_filter(rt: &ResourceRef) -> Option<String> {
    rt.filter.as_ref()?.labels.clone()
}
