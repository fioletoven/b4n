use backoff::backoff::Backoff;
use futures::{AsyncBufReadExt, TryStreamExt};
use k8s_openapi::{
    api::core::v1::Pod,
    chrono::{DateTime, Utc},
};
use kube::{Api, api::LogParams};
use std::time::Duration;
use thiserror;
use tokio::{
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
    time::sleep,
};
use tokio_util::sync::CancellationToken;

use crate::{
    app::utils::{build_default_backoff, wait_for_task},
    kubernetes::{Namespace, client::KubernetesClient},
};

/// Possible errors from [`LogsObserver`].
#[derive(thiserror::Error, Debug)]
pub enum LogsObserverError {
    /// Kubernetes client error.
    #[error("kubernetes client error")]
    KubeClientError(#[from] kube::Error),
}

pub struct PodRef {
    pub name: String,
    pub namespace: Namespace,
    pub container: Option<String>,
}

pub struct LogLine {
    pub datetime: DateTime<Utc>,
    pub message: String,
    pub is_error: bool,
}

pub struct LogsChunk {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub lines: Vec<LogLine>,
}

pub struct LogsObserver {
    task: Option<JoinHandle<()>>,
    cancellation_token: Option<CancellationToken>,
    context_tx: UnboundedSender<Box<LogsChunk>>,
    context_rx: UnboundedReceiver<Box<LogsChunk>>,
}

impl LogsObserver {
    pub fn new() -> Self {
        let (context_tx, context_rx) = mpsc::unbounded_channel();
        Self {
            task: None,
            cancellation_token: None,
            context_tx,
            context_rx,
        }
    }

    pub fn start(&mut self, client: &KubernetesClient, pod: PodRef) -> Result<(), LogsObserverError> {
        let cancellation_token = CancellationToken::new();
        let _cancellation_token = cancellation_token.clone();
        let _client = client.get_client();
        let _context_tx = self.context_tx.clone();

        let task = tokio::spawn(async move {
            let api: Api<Pod> = Api::namespaced(_client, pod.namespace.as_str());
            let mut backoff = build_default_backoff();

            let mut last_message = None;
            while !_cancellation_token.is_cancelled() {
                last_message = observe(&pod, &api, &_context_tx, last_message, &_cancellation_token).await;
                if _cancellation_token.is_cancelled() {
                    break;
                }

                tokio::select! {
                    _ = _cancellation_token.cancelled() => (),
                    _ = sleep(backoff.next_backoff().unwrap_or(Duration::from_millis(800))) => (),
                }
            }
        });

        self.cancellation_token = Some(cancellation_token);
        self.task = Some(task);

        Ok(())
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
        wait_for_task(self.task.take(), "logs");
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

async fn observe(
    pod: &PodRef,
    api: &Api<Pod>,
    channel: &UnboundedSender<Box<LogsChunk>>,
    last_message: Option<DateTime<Utc>>,
    cancellation_token: &CancellationToken,
) -> Option<DateTime<Utc>> {
    let mut params = LogParams {
        follow: true,
        container: pod.container.clone(),
        timestamps: true,
        ..LogParams::default()
    };

    if let Some(last_message) = last_message {
        params.since_time = Some(last_message);
    } else {
        params.tail_lines = Some(200);
    }

    let mut lines = match api.log_stream(&pod.name, &params).await {
        Ok(stream) => stream.lines(),
        Err(err) => {
            channel.send(Box::new(process_error(err.to_string()))).unwrap();
            return None;
        },
    };

    let mut last_message = None;
    while !cancellation_token.is_cancelled() {
        tokio::select! {
            _ = cancellation_token.cancelled() => (),
            line = lines.try_next() => {
                match line {
                    Ok(Some(line)) => {
                        if let Some(line) = process_line(line) {
                            last_message = Some(line.end);
                            channel.send(Box::new(line)).unwrap();
                        }
                    },
                    Ok(None) => {
                        channel.send(Box::new(process_error("Logs stream closed.".to_owned()))).unwrap();
                        break;
                    },
                    Err(err) => {
                        channel.send(Box::new(process_error(err.to_string()))).unwrap();
                        break;
                    },
                }
            },
        }
    }

    last_message
}

fn process_line(line: String) -> Option<LogsChunk> {
    let mut split = line.splitn(2, ' ');
    let dt = split.next()?.parse().ok()?;

    Some(LogsChunk {
        start: dt,
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
        start: dt,
        end: dt,
        lines: vec![LogLine {
            datetime: dt,
            message: error,
            is_error: true,
        }],
    }
}
