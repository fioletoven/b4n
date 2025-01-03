use kube::{
    api::ApiResource,
    discovery::{ApiCapabilities, Scope},
};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use thiserror;
use tokio::{
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
    time::sleep,
};
use tokio_util::sync::CancellationToken;
use tracing::{error, warn};

use crate::kubernetes::client::KubernetesClient;

use super::{utils::wait_for_task, ObserverResult};

/// Possible errors from [`BgObserver`]
#[derive(thiserror::Error, Debug)]
pub enum BgObserverError {
    /// [`BgObserver`] is already started.
    #[error("observer is already started")]
    AlreadyStarted,

    /// Resource was not found in k8s cluster
    #[error("kubernetes resource not found")]
    ResourceNotFound,
}

/// Background k8s resource observer
pub struct BgObserver {
    resource: String,
    namespace: Option<String>,
    scope: Scope,
    task: Option<JoinHandle<()>>,
    cancellation_token: Option<CancellationToken>,
    context_tx: UnboundedSender<ObserverResult>,
    context_rx: UnboundedReceiver<ObserverResult>,
    has_error: Arc<AtomicBool>,
}

impl BgObserver {
    /// Creates new [`BgObserver`] instance
    pub fn new() -> Self {
        let (context_tx, context_rx) = mpsc::unbounded_channel();
        BgObserver {
            resource: String::new(),
            namespace: None,
            scope: Scope::Cluster,
            task: None,
            cancellation_token: None,
            context_tx,
            context_rx,
            has_error: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Starts new [`BgObserver`] task
    pub fn start(
        &mut self,
        client: &KubernetesClient,
        resource_name: String,
        resource_namespace: Option<String>,
        discovery: Option<(ApiResource, ApiCapabilities)>,
    ) -> Result<Scope, BgObserverError> {
        if self.cancellation_token.is_some() {
            return Err(BgObserverError::AlreadyStarted);
        }

        let cancellation_token = CancellationToken::new();
        let (ar, cap) = discovery.ok_or(BgObserverError::ResourceNotFound)?;

        let _kind = ar.kind.clone();
        let _kind_plural = ar.plural.to_lowercase();
        let _group = ar.group.clone();
        self.scope = cap.scope.clone();
        let _scope = cap.scope.clone();

        let _api_client = client.get_api(ar, cap, resource_namespace.as_deref(), resource_namespace.is_none());

        let _cancellation_token = cancellation_token.clone();
        let _context_tx = self.context_tx.clone();

        self.has_error.store(false, Ordering::Relaxed);
        let _has_error = self.has_error.clone();

        let task = tokio::spawn(async move {
            while !_cancellation_token.is_cancelled() {
                let resources = _api_client.list(&Default::default()).await;
                match resources {
                    Ok(objects) => {
                        _context_tx
                            .send(ObserverResult::new(
                                _kind.clone(),
                                _kind_plural.clone(),
                                _group.clone(),
                                _scope.clone(),
                                objects,
                            ))
                            .unwrap();
                        _has_error.store(false, Ordering::Relaxed);
                    }
                    Err(error) => {
                        warn!("Cannot observe resource {}: {:?}", _kind_plural, error);
                        _has_error.store(true, Ordering::Relaxed);
                    }
                }

                tokio::select! {
                    _ = _cancellation_token.cancelled() => (),
                    _ = sleep(Duration::from_millis(2_000)) => (),
                }
            }
        });

        self.cancellation_token = Some(cancellation_token);
        self.task = Some(task);
        self.resource = resource_name;
        self.namespace = resource_namespace;

        Ok(self.scope.clone())
    }

    /// Restarts [`BgObserver`] task if `new_resource_name` or `new_namespace` is different than the current one
    pub fn restart(
        &mut self,
        client: &KubernetesClient,
        new_resource_name: String,
        new_namespace: Option<String>,
        discovery: Option<(ApiResource, ApiCapabilities)>,
    ) -> Result<Scope, BgObserverError> {
        if self.resource != new_resource_name || self.namespace != new_namespace {
            self.stop();
            self.drain();
            self.start(client, new_resource_name, new_namespace, discovery)?;
        }

        Ok(self.scope.clone())
    }

    /// Restarts [`BgObserver`] task if `new_resource_name` is different than the current one
    pub fn restart_new_kind(
        &mut self,
        client: &KubernetesClient,
        new_resource_name: String,
        discovery: Option<(ApiResource, ApiCapabilities)>,
    ) -> Result<Scope, BgObserverError> {
        if self.resource != new_resource_name {
            let mut namespace = None;
            if let Some((_, cap)) = &discovery {
                if cap.scope == Scope::Namespaced {
                    namespace = self.namespace.clone();
                }
            }

            self.stop();
            self.drain();
            self.start(client, new_resource_name, namespace, discovery)?;
        }

        Ok(self.scope.clone())
    }

    /// Restarts [`BgObserver`] task if `new_namespace` is different than the current one
    pub fn restart_new_namespace(
        &mut self,
        client: &KubernetesClient,
        new_namespace: Option<String>,
        discovery: Option<(ApiResource, ApiCapabilities)>,
    ) -> Result<Scope, BgObserverError> {
        if self.namespace != new_namespace {
            let resource = self.resource.clone();
            self.stop();
            self.drain();
            self.start(client, resource, new_namespace, discovery)?;
        }

        Ok(self.scope.clone())
    }

    /// Cancels [`BgObserver`] task
    pub fn cancel(&mut self) {
        if let Some(cancellation_token) = self.cancellation_token.take() {
            cancellation_token.cancel();
        }
    }

    /// Cancels [`BgObserver`] task and waits until it is finished
    pub fn stop(&mut self) {
        if let Some(cancellation_token) = self.cancellation_token.take() {
            cancellation_token.cancel();
            wait_for_task(self.task.take(), "discovery");
            self.resource = String::new();
            self.namespace = None;
        }
    }

    /// Tries to get next [`ObserverResult`]
    pub fn try_next(&mut self) -> Option<ObserverResult> {
        self.context_rx.try_recv().ok()
    }

    /// Drains waiting [`ObserverResult`]s
    pub fn drain(&mut self) {
        while let Ok(_) = self.context_rx.try_recv() {}
    }

    /// Returns currently observed resource name
    pub fn get_resource_name(&self) -> &str {
        &self.resource
    }

    /// Returns `true` if observer is in an error state
    pub fn has_error(&self) -> bool {
        self.has_error.load(Ordering::Relaxed)
    }
}

impl Drop for BgObserver {
    fn drop(&mut self) {
        self.cancel();
    }
}
