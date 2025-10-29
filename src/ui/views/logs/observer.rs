use b4n_kube::PodRef;
use b4n_kube::client::KubernetesClient;
use futures::{AsyncBufReadExt, TryStreamExt};
use k8s_openapi::{
    api::core::v1::Pod,
    chrono::{DateTime, Utc},
};
use kube::{Api, api::LogParams};
use std::time::Duration;
use thiserror;
use tokio::{
    runtime::Handle,
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
    time::sleep,
};
use tokio_util::sync::CancellationToken;

/// Possible errors from [`LogsObserver`].
#[derive(thiserror::Error, Debug)]
pub enum LogsObserverError {
    /// Kubernetes client error.
    #[error("kubernetes client error")]
    KubeClientError(#[from] kube::Error),
}

pub struct LogLine {
    pub datetime: DateTime<Utc>,
    pub message: String,
    pub is_error: bool,
}

pub struct LogsChunk {
    pub end: DateTime<Utc>,
    pub lines: Vec<LogLine>,
}

pub struct LogsObserver {
    runtime: Handle,
    task: Option<JoinHandle<()>>,
    cancellation_token: Option<CancellationToken>,
    context_tx: UnboundedSender<Box<LogsChunk>>,
    context_rx: UnboundedReceiver<Box<LogsChunk>>,
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

    pub fn start(&mut self, client: &KubernetesClient, pod: PodRef, tail_lines: Option<i64>, previous: bool) {
        let cancellation_token = CancellationToken::new();
        let _cancellation_token = cancellation_token.clone();
        let _client = client.get_client();
        let _context_tx = self.context_tx.clone();

        let task = self.runtime.spawn(async move {
            let api: Api<Pod> = Api::namespaced(_client, pod.namespace.as_str());
            let context = ObserverContext {
                pod: &pod,
                tail_lines,
                previous,
                api: &api,
                channel: &_context_tx,
                cancellation_token: &_cancellation_token,
            };

            let mut backoff = b4n_utils::ResettableBackoff::default();
            let mut since_time = None;
            let mut should_continue;
            while !_cancellation_token.is_cancelled() {
                (should_continue, since_time) = observe(since_time, &context).await;
                if _cancellation_token.is_cancelled() || !should_continue {
                    break;
                }

                tokio::select! {
                    () = _cancellation_token.cancelled() => (),
                    () = sleep(backoff.next_backoff().unwrap_or(Duration::from_millis(800))) => (),
                }
            }
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
        b4n_utils::tasks::wait_for_task(self.task.take(), "logs");
        self.drain();
    }

    /// Tries to get next [`LogsChunk`].
    pub fn try_next(&mut self) -> Option<Box<LogsChunk>> {
        self.context_rx.try_recv().ok()
    }

    /// Checks if [`LogsObserver`] is empty.
    pub fn is_empty(&self) -> bool {
        self.context_rx.is_empty()
    }

    /// Drains waiting [`LogsChunk`]s.
    pub fn drain(&mut self) {
        while self.context_rx.try_recv().is_ok() {}
    }
}

struct ObserverContext<'a> {
    pod: &'a PodRef,
    tail_lines: Option<i64>,
    previous: bool,
    api: &'a Api<Pod>,
    channel: &'a UnboundedSender<Box<LogsChunk>>,
    cancellation_token: &'a CancellationToken,
}

async fn observe(since_time: Option<DateTime<Utc>>, context: &ObserverContext<'_>) -> (bool, Option<DateTime<Utc>>) {
    let mut params = LogParams {
        follow: true,
        previous: context.previous,
        container: context.pod.container.clone(),
        timestamps: true,
        ..LogParams::default()
    };

    if let Some(since_time) = since_time {
        params.since_time = Some(since_time);
    } else {
        params.tail_lines = context.tail_lines;
    }

    let mut lines = match context.api.log_stream(&context.pod.name, &params).await {
        Ok(stream) => stream.lines(),
        Err(err) => {
            context.channel.send(Box::new(process_error(err.to_string()))).unwrap();
            return (true, None);
        },
    };

    let mut last_message_time = None;
    let mut should_continue = true;
    while !context.cancellation_token.is_cancelled() {
        tokio::select! {
            () = context.cancellation_token.cancelled() => (),
            line = lines.try_next() => {
                match line {
                    Ok(Some(line)) => {
                        if let Some(line) = process_line(&line) {
                            last_message_time = Some(line.end);
                            context.channel.send(Box::new(line)).unwrap();
                        }
                    },
                    Ok(None) => {
                        should_continue = false;
                        let msg = format!(
                            "Logs stream closed {}/{} ({})",
                            context.pod.namespace.as_str(),
                            context.pod.name,
                            context.pod.container.as_deref().unwrap_or_default());
                        context.channel.send(Box::new(process_error(msg))).unwrap();
                        break;
                    },
                    Err(err) => {
                        context.channel.send(Box::new(process_error(err.to_string()))).unwrap();
                        break;
                    },
                }
            },
        }
    }

    (should_continue, last_message_time)
}

fn process_line(line: &str) -> Option<LogsChunk> {
    let mut split = line.splitn(2, ' ');
    let dt = split.next()?.parse().ok()?;

    Some(LogsChunk {
        end: dt,
        lines: vec![LogLine {
            datetime: dt,
            message: split.next()?.to_owned(),
            is_error: false,
        }],
    })
}

fn process_error(error: String) -> LogsChunk {
    let dt = Utc::now();

    LogsChunk {
        end: dt,
        lines: vec![LogLine {
            datetime: dt,
            message: error,
            is_error: true,
        }],
    }
}
