use b4n_common::NotificationSink;
use futures::{StreamExt, TryStreamExt};
use kube::api::DynamicObject;
use kube::runtime::watcher::{self, DefaultBackoff, Error, Event, watcher};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;
use tokio_util::sync::CancellationToken;

use crate::watcher::{client::ResourceClient, result::ObserverResultSender, stream_backoff::StreamBackoff, utils};
use crate::{InitData, Namespace, ObserverResult};

const WATCH_ERROR_TIMEOUT_SECS: u64 = 120;

pub struct WatchInput {
    pub init_data: InitData,
    pub context_tx: ObserverResultSender,
    pub footer_tx: Option<NotificationSink>,
    pub is_connected: Arc<AtomicBool>,
    pub is_ready: Arc<AtomicBool>,
    pub has_error: Arc<AtomicBool>,
    pub has_access: Arc<AtomicBool>,
}

pub async fn watch(
    mut client: ResourceClient,
    input: WatchInput,
    fields: Option<String>,
    labels: Option<String>,
    mut fallback_namespace: Option<Namespace>,
    stop_on_access_error: bool,
    cancellation_token: CancellationToken,
) {
    let mut processor = EventsProcessor {
        init_data: input.init_data,
        context_tx: input.context_tx,
        footer_tx: input.footer_tx,
        stop_on_access_error: stop_on_access_error || fallback_namespace.is_some(),
        is_connected: input.is_connected,
        is_ready: input.is_ready,
        has_error: input.has_error,
        has_access: input.has_access,
        last_watch_error: None,
    };

    while !cancellation_token.is_cancelled() {
        let mut config = watcher::Config::default();
        if let Some(filter) = fields.as_ref() {
            config = config.fields(filter);
        }
        if let Some(filter) = labels.as_ref() {
            config = config.labels(filter);
        }

        let mut watch = StreamBackoff::new(watcher(client.get_api(), config), DefaultBackoff::default()).boxed();

        while !cancellation_token.is_cancelled() {
            tokio::select! {
                () = cancellation_token.cancelled() => (),
                result = watch.try_next() => {
                    match processor.process_event(result) {
                        ProcessorResult::Continue => (),
                        ProcessorResult::Restart => break, // we need to restart watcher, so go up one while loop
                        ProcessorResult::Stop => {
                            if let Some(ns) = fallback_namespace.take() {
                                processor.stop_on_access_error = stop_on_access_error;
                                client.set_namespace(ns);
                                break;
                            } else {
                                return;
                            }
                        },
                    }
                },
            }
        }
    }
}

/// Internal watcher's events processor result.
enum ProcessorResult {
    Continue,
    Restart,
    Stop,
}

/// Internal watcher's events processor.
struct EventsProcessor {
    init_data: InitData,
    context_tx: ObserverResultSender,
    footer_tx: Option<NotificationSink>,
    stop_on_access_error: bool,
    is_connected: Arc<AtomicBool>,
    is_ready: Arc<AtomicBool>,
    has_error: Arc<AtomicBool>,
    has_access: Arc<AtomicBool>,
    last_watch_error: Option<Instant>,
}

impl EventsProcessor {
    /// Process event received from the kubernetes resource watcher.\
    /// Returns `true` if all was OK or `false` if the watcher needs to be restarted.
    fn process_event(&mut self, result: Result<Option<Event<DynamicObject>>, Error>) -> ProcessorResult {
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
                        let _ = self.context_tx.send(Box::new(ObserverResult::InitDone));
                    },
                    Some(Event::InitApply(o) | Event::Apply(o)) => self.send_result(o, false),
                    Some(Event::Delete(o)) => self.send_result(o, true),
                    _ => (),
                }

                self.has_access.store(true, Ordering::Relaxed);
                if reset_error {
                    self.last_watch_error = None;
                    self.is_connected.store(true, Ordering::Relaxed);
                    self.has_error.store(false, Ordering::Relaxed);
                }
            },
            Err(error) => {
                let is_access_error = utils::is_api_error(&error, true);
                self.has_access.store(!is_access_error, Ordering::Relaxed);
                if self.stop_on_access_error && is_access_error {
                    self.is_connected.store(false, Ordering::Relaxed);
                    self.has_error.store(true, Ordering::Relaxed);
                    return ProcessorResult::Stop;
                }

                utils::log_error_message(
                    format!("Watch {}: {}", self.init_data.kind_plural, error),
                    self.footer_tx.as_ref(),
                );

                match error {
                    Error::WatchStartFailed(_) | Error::WatchFailed(_) => {
                        // WatchStartFailed and WatchFailed do not trigger Init, so we do not set error immediately.
                        if self
                            .last_watch_error
                            .is_some_and(|t| t.elapsed().as_secs() <= WATCH_ERROR_TIMEOUT_SECS)
                        {
                            tracing::warn!("Forcefully restarting watcher for {}", self.init_data.kind_plural);
                            self.is_connected.store(utils::is_api_error(&error, false), Ordering::Relaxed);
                            self.has_error.store(true, Ordering::Relaxed);
                            self.last_watch_error = Some(Instant::now());

                            return ProcessorResult::Restart;
                        }

                        self.last_watch_error = Some(Instant::now());
                    },
                    _ => {
                        self.is_connected.store(utils::is_api_error(&error, false), Ordering::Relaxed);
                        self.has_error.store(true, Ordering::Relaxed);
                    },
                }
            },
        }

        ProcessorResult::Continue
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
