use futures::{AsyncBufReadExt, TryStreamExt};
use k8s_openapi::api::core::v1::Pod;
use kube::{Api, api::LogParams};
use thiserror;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use tokio::{
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;
use tracing::warn;

use crate::{
    app::utils::wait_for_task,
    kubernetes::{Namespace, client::KubernetesClient},
};

/// Possible errors from [`LogsObserver`].
#[derive(thiserror::Error, Debug)]
pub enum LogsObserverError {
    /// Resource was not found in k8s cluster
    #[error("kubernetes client error")]
    KubeClientError(#[from] kube::Error),
}

pub struct PodRef {
    pub name: String,
    pub namespace: Namespace,
    pub container: Option<String>,
}

pub struct LogLine {
    pub datetime: OffsetDateTime,
    pub message: String,
}

pub struct LogsChunk {
    pub start: OffsetDateTime,
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
            let pods: Api<Pod> = Api::namespaced(_client, pod.namespace.as_str());
            let params = LogParams {
                follow: true,
                container: pod.container,
                tail_lines: Some(200),
                timestamps: true,
                ..LogParams::default()
            };

            let mut lines = match pods.log_stream(&pod.name, &params).await {
                Ok(stream) => stream.lines(),
                Err(err) => {
                    warn!("Error while initialising logs stream: {}", err);
                    return;
                },
            };

            while !_cancellation_token.is_cancelled() {
                tokio::select! {
                    _ = _cancellation_token.cancelled() => (),
                    line = lines.try_next() => {
                        match line {
                            Ok(Some(line)) => {
                                if let Some(line) = process_line(line) {
                                    _context_tx.send(Box::new(line)).unwrap();
                                }
                            },
                            Ok(None) => return,
                            Err(err) => {
                                warn!("Error while reading logs stream: {}", err);
                                return;
                            },
                        }
                    },
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

fn process_line(line: String) -> Option<LogsChunk> {
    let mut split = line.splitn(2, ' ');
    let dt = OffsetDateTime::parse(split.next()?, &Rfc3339).ok()?;

    Some(LogsChunk {
        start: dt,
        lines: vec![LogLine {
            datetime: dt,
            message: split.next()?.to_owned(),
        }],
    })
}
