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
};
use thiserror;
use tokio::{
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;
use tracing::{error, warn};

use crate::{
    kubernetes::{Namespace, client::KubernetesClient},
    ui::widgets::FooterMessage,
};

use super::utils::wait_for_task;

/// Possible errors from [`BgObserver`].
#[derive(thiserror::Error, Debug)]
pub enum BgObserverError {
    /// Resource was not found in k8s cluster
    #[error("kubernetes resource not found")]
    ResourceNotFound,
}

/// Background observer result.
pub struct ObserverResult {
    pub init: Option<ObserverInitData>,
    pub object: Option<DynamicObject>,
    pub is_delete: bool,
}

/// Data that is returned when [`BgObserver`] starts watching resource.
pub struct ObserverInitData {
    pub kind: String,
    pub kind_plural: String,
    pub group: String,
    pub scope: Scope,
}

impl ObserverInitData {
    /// Creates new [`ObserverResult`] initial data.
    pub fn new(kind: String, kind_plural: String, group: String, scope: Scope) -> Self {
        ObserverInitData {
            kind,
            kind_plural,
            group,
            scope,
        }
    }
}

/// Background k8s resource observer.
pub struct BgObserver {
    resource: String,
    namespace: Namespace,
    scope: Scope,
    task: Option<JoinHandle<()>>,
    cancellation_token: Option<CancellationToken>,
    context_tx: UnboundedSender<ObserverResult>,
    context_rx: UnboundedReceiver<ObserverResult>,
    footer_tx: UnboundedSender<FooterMessage>,
    has_error: Arc<AtomicBool>,
}

impl BgObserver {
    /// Creates new [`BgObserver`] instance.
    pub fn new(footer_tx: UnboundedSender<FooterMessage>) -> Self {
        let (context_tx, context_rx) = mpsc::unbounded_channel();
        Self {
            resource: String::new(),
            namespace: Namespace::default(),
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
        resource_name: String,
        resource_namespace: Namespace,
        discovery: Option<(ApiResource, ApiCapabilities)>,
    ) -> Result<Scope, BgObserverError> {
        self.stop();

        let cancellation_token = CancellationToken::new();
        let (ar, cap) = discovery.ok_or(BgObserverError::ResourceNotFound)?;

        let _kind = ar.kind.clone();
        let _kind_plural = ar.plural.to_lowercase();
        let _group = ar.group.clone();
        self.scope = cap.scope.clone();
        let _scope = cap.scope.clone();

        let _api_client = client.get_api(ar, cap, resource_namespace.as_option(), resource_namespace.is_all());

        let _cancellation_token = cancellation_token.clone();
        let _context_tx = self.context_tx.clone();

        self.has_error.store(false, Ordering::Relaxed);
        let _has_error = Arc::clone(&self.has_error);
        let _footer_tx = self.footer_tx.clone();

        let task = tokio::spawn(async move {
            let watch = watcher(_api_client, watcher::Config::default()).default_backoff();
            let mut watch = pin!(watch);

            while !_cancellation_token.is_cancelled() {
                tokio::select! {
                    _ = _cancellation_token.cancelled() => (),
                    result = watch.try_next() => {
                        match result {
                            Ok(event) => {
                                if let Some(result) = match event {
                                    Some(Event::Init) => Some(get_init_result(
                                        _kind.clone(),
                                        _kind_plural.clone(),
                                        _group.clone(),
                                        _scope.clone())),
                                    Some(Event::InitApply(o) | Event::Apply(o)) => Some(get_result(o, false)),
                                    Some(Event::Delete(o)) => Some(get_result(o, true)),
                                    _ => None,
                                } {
                                    _context_tx.send(result).unwrap();
                                }
                                _has_error.store(false, Ordering::Relaxed);
                            },
                            Err(error) => {
                                let msg = format!("Watch {}: {}", _kind_plural, error);
                                warn!("{}", msg);
                                _footer_tx.send(FooterMessage::error(msg, 0)).unwrap();
                                match error {
                                    Error::WatchFailed(_) => (), // WatchFailed do not trigger Init, so we do not set _has_error.
                                    _ =>_has_error.store(true, Ordering::Relaxed),
                                }
                            }
                        };
                    },
                }
            }
        });

        self.cancellation_token = Some(cancellation_token);
        self.task = Some(task);
        self.resource = resource_name;
        self.namespace = resource_namespace;

        Ok(self.scope.clone())
    }

    /// Restarts [`BgObserver`] task if `new_resource_name` or `new_namespace` is different than the current one.
    pub fn restart(
        &mut self,
        client: &KubernetesClient,
        new_resource_name: String,
        new_namespace: Namespace,
        discovery: Option<(ApiResource, ApiCapabilities)>,
    ) -> Result<Scope, BgObserverError> {
        if self.resource != new_resource_name || self.namespace != new_namespace {
            self.start(client, new_resource_name, new_namespace, discovery)?;
        }

        Ok(self.scope.clone())
    }

    /// Restarts [`BgObserver`] task if `new_resource_name` is different from the current one.
    /// **Note** that it uses `new_namespace` if resource is namespaced.
    pub fn restart_new_kind(
        &mut self,
        client: &KubernetesClient,
        new_kind: String,
        new_namespace: Namespace,
        discovery: Option<(ApiResource, ApiCapabilities)>,
    ) -> Result<Scope, BgObserverError> {
        if self.resource != new_kind {
            let mut namespace = Namespace::all();
            if let Some((_, cap)) = &discovery {
                if cap.scope == Scope::Namespaced {
                    namespace = new_namespace;
                }
            }

            self.start(client, new_kind, namespace, discovery)?;
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
        if self.namespace != new_namespace {
            self.start(client, self.resource.clone(), new_namespace, discovery)?;
        }

        Ok(self.scope.clone())
    }

    /// Cancels [`BgObserver`] task.
    pub fn cancel(&mut self) {
        if let Some(cancellation_token) = self.cancellation_token.take() {
            cancellation_token.cancel();
            self.resource = String::new();
            self.namespace = Namespace::default();
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
    pub fn try_next(&mut self) -> Option<ObserverResult> {
        self.context_rx.try_recv().ok()
    }

    /// Drains waiting [`ObserverResult`]s.
    pub fn drain(&mut self) {
        while self.context_rx.try_recv().is_ok() {}
    }

    /// Returns currently observed resource name.
    pub fn get_resource_name(&self) -> &str {
        &self.resource
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

fn get_init_result(kind: String, kind_plural: String, group: String, scope: Scope) -> ObserverResult {
    ObserverResult {
        init: Some(ObserverInitData::new(kind, kind_plural, group, scope)),
        object: None,
        is_delete: false,
    }
}

fn get_result(object: DynamicObject, is_delete: bool) -> ObserverResult {
    ObserverResult {
        init: None,
        object: Some(object),
        is_delete,
    }
}
