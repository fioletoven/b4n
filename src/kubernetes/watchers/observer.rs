use b4n_kube::{CONTAINERS, Kind};
use futures::TryStreamExt;
use kube::{
    Api,
    api::{ApiResource, DynamicObject, ListParams, ObjectList},
    discovery::{ApiCapabilities, Scope, verbs},
    runtime::{
        WatchStreamExt,
        watcher::{self, Error, Event, watcher},
    },
};
use std::{
    collections::HashMap,
    pin::pin,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use thiserror;
use tokio::{
    runtime::Handle,
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
    time::sleep,
};
use tokio_util::sync::CancellationToken;
use tracing::{error, warn};
use uuid::Uuid;

use crate::{
    kubernetes::{ResourceRef, client::KubernetesClient, resources::CrdColumns, utils::get_object_uid},
    ui::widgets::FooterTx,
};

const WATCH_ERROR_TIMEOUT_SECS: u64 = 120;

/// Possible errors from [`BgObserver`].
#[derive(thiserror::Error, Debug)]
pub enum BgObserverError {
    /// Resource was not found in k8s cluster
    #[error("kubernetes resource not found")]
    ResourceNotFound,

    /// Resource cannot be watched or listed
    #[error("resource cannot be watched or listed")]
    UnsupportedOperation,
}

/// Background observer result.
pub enum ObserverResult<T> {
    Init(Box<InitData>),
    InitDone,
    Apply(T),
    Delete(T),
}

impl<T> ObserverResult<T> {
    /// Creates new [`ObserverResult`] for resource.
    pub fn new(resource: T, is_delete: bool) -> Self {
        if is_delete {
            Self::Delete(resource)
        } else {
            Self::Apply(resource)
        }
    }
}

/// Data that is returned when [`BgObserver`] starts watching resource.
#[derive(Clone)]
pub struct InitData {
    pub uuid: String,
    pub resource: ResourceRef,
    pub kind: String,
    pub kind_plural: String,
    pub group: String,
    pub version: String,
    pub scope: Scope,
    pub crd: Option<CrdColumns>,
    pub has_metrics: bool,
}

impl Default for InitData {
    fn default() -> Self {
        Self {
            uuid: String::new(),
            resource: ResourceRef::default(),
            kind: String::new(),
            kind_plural: String::new(),
            group: String::new(),
            version: String::new(),
            scope: Scope::Cluster,
            crd: None,
            has_metrics: false,
        }
    }
}

impl InitData {
    /// Creates new initial data for [`ObserverResult`].
    fn new(rt: &ResourceRef, ar: &ApiResource, scope: Scope, crd: Option<CrdColumns>, has_metrics: bool) -> Self {
        let kind = if rt.is_container() { "Container" } else { ar.kind.as_str() };
        let kind_plural = if rt.is_container() { CONTAINERS } else { ar.plural.as_str() };
        Self {
            uuid: Uuid::new_v4()
                .hyphenated()
                .encode_lower(&mut Uuid::encode_buffer())
                .to_owned(),
            resource: rt.clone(),
            kind: kind.to_owned(),
            kind_plural: kind_plural.to_lowercase(),
            group: ar.group.clone(),
            version: ar.version.clone(),
            scope,
            crd,
            has_metrics,
        }
    }
}

type ObserverResultSender = UnboundedSender<Box<ObserverResult<DynamicObject>>>;
type ObserverResultReceiver = UnboundedReceiver<Box<ObserverResult<DynamicObject>>>;

/// Background k8s resource observer.
pub struct BgObserver {
    pub resource: ResourceRef,
    pub init: Option<InitData>,
    pub scope: Scope,
    runtime: Handle,
    task: Option<JoinHandle<()>>,
    cancellation_token: Option<CancellationToken>,
    context_tx: ObserverResultSender,
    context_rx: ObserverResultReceiver,
    footer_tx: FooterTx,
    is_ready: Arc<AtomicBool>,
    has_error: Arc<AtomicBool>,
}

impl BgObserver {
    /// Creates new [`BgObserver`] instance.
    pub fn new(runtime: Handle, footer_tx: FooterTx) -> Self {
        let (context_tx, context_rx) = mpsc::unbounded_channel();
        Self {
            resource: ResourceRef::default(),
            init: None,
            scope: Scope::Cluster,
            runtime,
            task: None,
            cancellation_token: None,
            context_tx,
            context_rx,
            footer_tx,
            is_ready: Arc::new(AtomicBool::new(false)),
            has_error: Arc::new(AtomicBool::new(true)),
        }
    }

    /// Starts new [`BgObserver`] task.\
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
        self.is_ready.store(false, Ordering::Relaxed);
        self.has_error.store(false, Ordering::Relaxed);

        let init_data = InitData::new(&self.resource, &ar, cap.scope.clone(), None, false);
        let api_client = client.get_api(
            &ar,
            &cap,
            self.resource.namespace.as_option(),
            self.resource.namespace.is_all(),
        );

        let task = if cap.supports_operation(verbs::WATCH) {
            self.watch(api_client, init_data.clone(), cancellation_token.clone())
        } else if cap.supports_operation(verbs::LIST) {
            self.list(api_client, init_data.clone(), cancellation_token.clone())
        } else {
            return Err(BgObserverError::UnsupportedOperation);
        };

        self.cancellation_token = Some(cancellation_token);
        self.task = Some(task);
        self.init = Some(init_data);

        Ok(self.scope.clone())
    }

    /// Restarts [`BgObserver`] task if `new_resource` is different from the current one.
    pub fn restart(
        &mut self,
        client: &KubernetesClient,
        new_resource: ResourceRef,
        discovery: Option<(ApiResource, ApiCapabilities)>,
    ) -> Result<Scope, BgObserverError> {
        if self.resource != new_resource {
            self.start(client, new_resource, discovery)?;
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
        b4n_utils::tasks::wait_for_task(self.task.take(), "background observer");
        self.drain();
    }

    /// Tries to get next [`ObserverResult`].
    pub fn try_next(&mut self) -> Option<Box<ObserverResult<DynamicObject>>> {
        self.context_rx.try_recv().ok()
    }

    /// Drains waiting [`ObserverResult`]s.
    pub fn drain(&mut self) {
        while self.context_rx.try_recv().is_ok() {}
    }

    /// Returns currently observed resource kind.
    pub fn get_resource_kind(&self) -> &Kind {
        &self.resource.kind
    }

    /// Returns `true` if the observed resource is a container.
    pub fn is_container(&self) -> bool {
        self.resource.is_container()
    }

    /// Returns `true` if the observed resource is filtered.
    pub fn is_filtered(&self) -> bool {
        self.resource.filter.is_some()
    }

    /// Returns `true` if the observer has received the initial list of resources.
    pub fn is_ready(&self) -> bool {
        self.is_ready.load(Ordering::Relaxed)
    }

    /// Returns `true` if the observer is not running or is in an error state.
    pub fn has_error(&self) -> bool {
        self.has_error.load(Ordering::Relaxed)
    }

    fn watch(
        &mut self,
        client: Api<DynamicObject>,
        init_data: InitData,
        cancellation_token: CancellationToken,
    ) -> JoinHandle<()> {
        self.runtime.spawn({
            let mut processor = EventsProcessor {
                init_data,
                context_tx: self.context_tx.clone(),
                footer_tx: self.footer_tx.clone(),
                is_ready: Arc::clone(&self.is_ready),
                has_error: Arc::clone(&self.has_error),
                last_watch_error: None,
            };
            let fields = build_fields_filter(&self.resource);
            let labels = build_labels_filter(&self.resource);

            async move {
                while !cancellation_token.is_cancelled() {
                    let mut config = watcher::Config::default();
                    if let Some(filter) = fields.as_ref() {
                        config = config.fields(filter);
                    }
                    if let Some(filter) = labels.as_ref() {
                        config = config.labels(filter);
                    }
                    let watch = watcher(client.clone(), config).default_backoff();
                    let mut watch = pin!(watch);

                    while !cancellation_token.is_cancelled() {
                        tokio::select! {
                            () = cancellation_token.cancelled() => (),
                            result = watch.try_next() => {
                                if !processor.process_event(result) {
                                    // we need to restart watcher, so go up one while loop
                                    break;
                                }
                            },
                        }
                    }
                }
            }
        })
    }

    fn list(&mut self, client: Api<DynamicObject>, init_data: InitData, cancellation_token: CancellationToken) -> JoinHandle<()> {
        self.runtime.spawn({
            let is_ready = Arc::clone(&self.is_ready);
            let has_error = Arc::clone(&self.has_error);
            let context_tx = self.context_tx.clone();
            let fields = build_fields_filter(&self.resource);
            let labels = build_labels_filter(&self.resource);
            let mut results = None;

            async move {
                let mut params = ListParams::default();
                if let Some(filter) = fields.as_ref() {
                    params = params.fields(filter);
                }
                if let Some(filter) = labels.as_ref() {
                    params = params.labels(filter);
                }

                while !cancellation_token.is_cancelled() {
                    let resources = client.list(&params).await;
                    match resources {
                        Ok(objects) => {
                            results = Some(emit_results(objects, results, &init_data, &context_tx));
                            is_ready.store(true, Ordering::Relaxed);
                            has_error.store(false, Ordering::Relaxed);
                        },
                        Err(error) => {
                            results = None;
                            warn!("Cannot list resource {}: {:?}", init_data.kind_plural, error);
                            is_ready.store(false, Ordering::Relaxed);
                            has_error.store(true, Ordering::Relaxed);
                        },
                    }

                    tokio::select! {
                        () = cancellation_token.cancelled() => (),
                        () = sleep(Duration::from_millis(5_000)) => (),
                    }
                }
            }
        })
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

fn emit_results(
    objects: ObjectList<DynamicObject>,
    prev_results: Option<HashMap<String, DynamicObject>>,
    init_data: &InitData,
    context_tx: &ObserverResultSender,
) -> HashMap<String, DynamicObject> {
    let result = objects.items.iter().map(|o| (get_object_uid(o), o.clone())).collect();
    if let Some(mut prev_results) = prev_results {
        for object in objects {
            prev_results.remove(&get_object_uid(&object));
            let _ = context_tx.send(Box::new(ObserverResult::new(object, false)));
        }

        for (_, object) in prev_results {
            let _ = context_tx.send(Box::new(ObserverResult::new(object, true)));
        }
    } else {
        let _ = context_tx.send(Box::new(ObserverResult::Init(Box::new(init_data.clone()))));
        for object in objects {
            let _ = context_tx.send(Box::new(ObserverResult::new(object, false)));
        }

        let _ = context_tx.send(Box::new(ObserverResult::InitDone));
    }

    result
}

impl Drop for BgObserver {
    fn drop(&mut self) {
        self.cancel();
    }
}

/// Internal watcher's events processor.
struct EventsProcessor {
    init_data: InitData,
    context_tx: ObserverResultSender,
    footer_tx: FooterTx,
    is_ready: Arc<AtomicBool>,
    has_error: Arc<AtomicBool>,
    last_watch_error: Option<Instant>,
}

impl EventsProcessor {
    /// Process event received from the kubernetes resource watcher.\
    /// Returns `true` if all was OK or `false` if the watcher needs to be restarted.
    pub fn process_event(&mut self, result: Result<Option<Event<DynamicObject>>, Error>) -> bool {
        match result {
            Ok(event) => {
                let mut reset_error = true;
                match event {
                    Some(Event::Init) => {
                        reset_error = false; // Init is also emitted after a forced restart of the watcher
                        self.is_ready.store(false, Ordering::Relaxed);
                        self.send_init_result();
                    },
                    Some(Event::InitDone) => {
                        self.is_ready.store(true, Ordering::Relaxed);
                        self.context_tx.send(Box::new(ObserverResult::InitDone)).unwrap();
                    },
                    Some(Event::InitApply(o) | Event::Apply(o)) => self.send_result(o, false),
                    Some(Event::Delete(o)) => self.send_result(o, true),
                    _ => (),
                }

                if reset_error {
                    self.last_watch_error = None;
                    self.has_error.store(false, Ordering::Relaxed);
                }
            },
            Err(error) => {
                let msg = format!("Watch {}: {}", self.init_data.kind_plural, error);
                warn!("{}", msg);
                self.footer_tx.show_error(msg, 0);

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
                        }

                        self.last_watch_error = Some(Instant::now());
                    },
                    _ => self.has_error.store(true, Ordering::Relaxed),
                }
            },
        }

        true
    }

    fn send_init_result(&self) {
        let _ = self
            .context_tx
            .send(Box::new(ObserverResult::Init(Box::new(self.init_data.clone()))));
    }

    fn send_result(&self, object: DynamicObject, is_delete: bool) {
        let _ = self.context_tx.send(Box::new(ObserverResult::new(object, is_delete)));
    }
}
