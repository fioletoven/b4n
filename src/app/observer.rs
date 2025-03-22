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
pub struct ObserverResult {
    pub init: Option<ObserverInitData>,
    pub object: Option<Resource>,
    pub is_delete: bool,
}

/// Data that is returned when [`BgObserver`] starts watching resource.
#[derive(Clone)]
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

        self.scope = cap.scope.clone();
        self.has_error.store(false, Ordering::Relaxed);

        let mut _processor = EventsProcessor {
            init_data: ObserverInitData::new(ar.kind.clone(), ar.plural.to_lowercase(), ar.group.clone(), cap.scope.clone()),
            context_tx: self.context_tx.clone(),
            footer_tx: self.footer_tx.clone(),
            has_error: Arc::clone(&self.has_error),
            last_watch_error: None,
        };
        let _api_client = client.get_api(ar, cap, resource_namespace.as_option(), resource_namespace.is_all());
        let _cancellation_token = cancellation_token.clone();

        let task = tokio::spawn(async move {
            while !_cancellation_token.is_cancelled() {
                let watch = watcher(_api_client.clone(), watcher::Config::default()).default_backoff();
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
    pub fn try_next(&mut self) -> Option<Box<ObserverResult>> {
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

/// Internal watcher's events processor.
struct EventsProcessor {
    pub init_data: ObserverInitData,
    pub context_tx: UnboundedSender<Box<ObserverResult>>,
    pub footer_tx: UnboundedSender<FooterMessage>,
    pub has_error: Arc<AtomicBool>,
    pub last_watch_error: Option<Instant>,
}

impl EventsProcessor {
    /// Process watcher's event. It returns `true` if all was OK or `false` if the watcher needs to be restarted.
    pub fn process_event(&mut self, result: Result<Option<Event<DynamicObject>>, Error>) -> bool {
        match result {
            Ok(event) => {
                let mut reset_error = true;
                if let Some(result) = match event {
                    Some(Event::Init) => {
                        reset_error = false; // Init is also emitted after a forced restart of the watcher
                        Some(self.build_init_result())
                    }
                    Some(Event::InitApply(o) | Event::Apply(o)) => Some(self.build_result(o, false)),
                    Some(Event::Delete(o)) => Some(self.build_result(o, true)),
                    _ => None,
                } {
                    self.context_tx.send(result).unwrap();
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

    fn build_init_result(&self) -> Box<ObserverResult> {
        Box::new(ObserverResult {
            init: Some(self.init_data.clone()),
            object: None,
            is_delete: false,
        })
    }

    fn build_result(&self, object: DynamicObject, is_delete: bool) -> Box<ObserverResult> {
        Box::new(ObserverResult {
            init: None,
            object: Some(Resource::from(&self.init_data.kind, object)),
            is_delete,
        })
    }
}
