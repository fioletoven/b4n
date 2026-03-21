use b4n_kube::ContainerRef;
use b4n_kube::client::KubernetesClient;
use futures::{AsyncBufReadExt, TryStreamExt};
use k8s_openapi::api::core::v1::Pod;
use k8s_openapi::jiff::{SignedDuration, Timestamp};
use kube::{Api, api::LogParams, runtime::watcher::DefaultBackoff};
use std::time::Duration;
use tokio::runtime::Handle;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

use crate::ui::views::logs::line::LogLine;

/// Possible errors from [`LogsObserver`].
#[derive(thiserror::Error, Debug)]
pub enum LogsObserverError {
    /// Kubernetes client error.
    #[error("kubernetes client error")]
    KubeClientError(#[from] kube::Error),
}

/// Options for [`LogsObserver`].
#[derive(Default)]
pub struct LogsObserverOptions {
    previous: bool,
    since_time: Option<Timestamp>,
    tail_lines: Option<i64>,
    include_container: bool,
    stop_on: Option<(Timestamp, String)>,
}

impl LogsObserverOptions {
    /// Creates new options for the endless logs observer.
    pub fn new(tail_lines: Option<i64>, include_container: bool, previous: bool) -> Self {
        Self {
            previous,
            tail_lines,
            include_container,
            ..Default::default()
        }
    }

    /// Creates new options for the logs observer that will stop on a specified logs line.
    pub fn stop_on(since_time: Timestamp, stop_on: (Timestamp, String), previous: bool) -> Self {
        Self {
            previous,
            since_time: Some(since_time),
            stop_on: Some(stop_on),
            ..Default::default()
        }
    }
}

/// Kubernetes container logs observer.
pub struct LogsObserver {
    runtime: Handle,
    task: Option<JoinHandle<()>>,
    cancellation_token: Option<CancellationToken>,
    context_tx: UnboundedSender<Box<LogLine>>,
    context_rx: UnboundedReceiver<Box<LogLine>>,
}

impl LogsObserver {
    pub fn new(runtime: Handle) -> Self {
        let (context_tx, context_rx) = mpsc::unbounded_channel();
        Self {
            runtime,
            task: None,
            cancellation_token: None,
            context_tx,
            context_rx,
        }
    }

    pub fn start(&mut self, client: &KubernetesClient, container: ContainerRef, options: LogsObserverOptions) {
        let cancellation_token = CancellationToken::new();
        let _cancellation_token = cancellation_token.clone();
        let _client = client.get_client();
        let _context_tx = self.context_tx.clone();

        let task = self.runtime.spawn(async move {
            let api: Api<Pod> = Api::namespaced(_client, container.namespace.as_str());
            let context = ObserverContext {
                pod: &container,
                tail_lines: options.tail_lines,
                previous: options.previous,
                include_container: options.include_container,
                api: &api,
                channel: &_context_tx,
                cancellation_token: &_cancellation_token,
                stop_on: options.stop_on,
            };

            let mut backoff = DefaultBackoff::default();
            let mut since_time = options.since_time;
            let mut should_continue = ObserveResult::Continue;
            while !_cancellation_token.is_cancelled() {
                (should_continue, since_time) = observe(since_time, &context).await;
                if _cancellation_token.is_cancelled() || should_continue != ObserveResult::Continue {
                    break;
                }

                tokio::select! {
                    () = _cancellation_token.cancelled() => (),
                    () = sleep(backoff.next().unwrap_or(Duration::from_millis(800))) => (),
                }
            }

            if should_continue == ObserveResult::StopOn {
                return;
            }

            let msg = format!(
                "Logs stream closed {}/{} ({})",
                context.pod.namespace.as_str(),
                context.pod.name,
                context.pod.container.as_deref().unwrap_or_default()
            );
            let msg_time = container
                .finished_at
                .and_then(|t| t.checked_add(SignedDuration::from_secs(1)).ok());
            let container = if options.include_container {
                container.container.as_deref()
            } else {
                None
            };

            context.send_log_line(process_error(container, msg, msg_time));
        });

        self.cancellation_token = Some(cancellation_token);
        self.task = Some(task);
    }

    /// Cancels [`LogsObserver`] task.
    pub fn cancel(&mut self) {
        if let Some(cancellation_token) = self.cancellation_token.take() {
            cancellation_token.cancel();
        }
    }

    /// Cancels [`LogsObserver`] task and waits until it is finished.
    pub fn stop(&mut self) {
        self.cancel();
        b4n_common::tasks::wait_for_task(self.task.take(), "logs");
        self.drain();
    }

    /// Tries to get next [`LogLine`].
    pub fn try_next(&mut self) -> Option<Box<LogLine>> {
        self.context_rx.try_recv().ok()
    }

    /// Checks if [`LogsObserver`] is empty.
    pub fn is_empty(&self) -> bool {
        self.context_rx.is_empty()
    }

    /// Drains waiting [`LogLine`]s.
    pub fn drain(&mut self) {
        while self.context_rx.try_recv().is_ok() {}
    }

    /// Returns `true` if observer finished watching logs.
    pub fn is_finished(&self) -> bool {
        self.task.as_ref().is_some_and(|t| t.is_finished())
    }
}

struct ObserverContext<'a> {
    pod: &'a ContainerRef,
    tail_lines: Option<i64>,
    previous: bool,
    include_container: bool,
    api: &'a Api<Pod>,
    channel: &'a UnboundedSender<Box<LogLine>>,
    cancellation_token: &'a CancellationToken,
    stop_on: Option<(Timestamp, String)>,
}

impl ObserverContext<'_> {
    /// Sends [`LogLine`] to the channel.
    fn send_log_line(&self, line: LogLine) {
        let _ = self.channel.send(Box::new(line));
    }
}

#[derive(PartialEq)]
enum ObserveResult {
    Continue,
    Stop,
    StopOn,
}

async fn observe(since_time: Option<Timestamp>, context: &ObserverContext<'_>) -> (ObserveResult, Option<Timestamp>) {
    let mut params = LogParams {
        follow: true,
        previous: context.previous,
        container: context.pod.container.clone(),
        timestamps: true,
        ..LogParams::default()
    };

    let container = if context.include_container {
        context.pod.container.as_deref()
    } else {
        None
    };

    if let Some(since_time) = since_time {
        params.since_time = Some(since_time);
    } else {
        params.tail_lines = context.tail_lines;
    }

    let mut lines = match context.api.log_stream(&context.pod.name, &params).await {
        Ok(stream) => stream.lines(),
        Err(err) => {
            context.send_log_line(process_error(container, err.to_string(), None));
            return (ObserveResult::Continue, since_time);
        },
    };

    let mut last_message_time = since_time;
    let mut result = ObserveResult::Continue;
    while !context.cancellation_token.is_cancelled() {
        tokio::select! {
            () = context.cancellation_token.cancelled() => (),
            line = lines.try_next() => {
                match line {
                    Ok(Some(line)) => {
                        if let Some(line) = process_line(container, &line) {
                            last_message_time = Some(line.datetime);

                            if context.stop_on.as_ref().is_some_and(|s| should_stop_on(&line, s.0, &s.1)) {
                                result = ObserveResult::StopOn;
                                break;
                            }

                            context.send_log_line(line);
                        }
                    },
                    Ok(None) => {
                        result = ObserveResult::Stop;
                        break;
                    },
                    Err(err) => {
                        context.send_log_line(process_error(container, err.to_string(), None));
                        break;
                    },
                }
            },
        }
    }

    (result, last_message_time)
}

fn process_line(container: Option<&str>, line: &str) -> Option<LogLine> {
    let mut split = line.splitn(2, ' ');
    let dt = split.next()?.parse().ok()?;
    let msg = split.next()?.replace('\t', "    ");

    Some(LogLine::new(dt, container, msg))
}

fn process_error(container: Option<&str>, error: String, dt: Option<Timestamp>) -> LogLine {
    let dt = dt.unwrap_or_else(Timestamp::now);
    LogLine::error(dt, container, error)
}

fn should_stop_on(current: &LogLine, dt: Timestamp, log: &str) -> bool {
    current.datetime == dt && current.lowercase == log
}
