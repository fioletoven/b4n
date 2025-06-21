use k8s_openapi::api::core::v1::Pod;
use kube::Api;
use std::{
    net::SocketAddr,
    sync::{
        Arc,
        atomic::{AtomicI16, AtomicI32, Ordering},
    },
};
use tokio::{
    net::TcpListener,
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;
use tracing::warn;

use crate::{
    kubernetes::{ResourceRef, client::KubernetesClient, resources::PODS},
    ui::widgets::FooterMessage,
};

/// Possible errors from [`PortForwarder`].
#[derive(thiserror::Error, Debug)]
pub enum PortForwardError {
    /// Provided resource is not a named pod.
    #[error("unsupported resource")]
    UnsupportedResource,

    /// Provided port is not found in the pod.
    #[error("port not found in pod")]
    PortNotFound,

    /// Forwarding stream I/O error.
    #[error("stream I/O error: {0}")]
    IoError(#[from] std::io::Error),

    /// Kubernetes client error.
    #[error("kube client error: {0}")]
    KubeError(#[from] kube::Error),

    /// Portforward task error.
    #[error("{0}")]
    PortforwardError(String),
}

pub enum PortForwardEvent {
    TaskStarted,
    TaskStopped,
    ConnectionAccepted,
    ConnectionClosed,
    ConnectionError,
}

pub struct PortForwarder {
    tasks: Vec<PortForwardTask>,
    events_tx: UnboundedSender<PortForwardEvent>,
    events_rx: UnboundedReceiver<PortForwardEvent>,
    footer_tx: UnboundedSender<FooterMessage>,
}

impl PortForwarder {
    pub fn new(footer_tx: UnboundedSender<FooterMessage>) -> Self {
        let (events_tx, events_rx) = mpsc::unbounded_channel();
        Self {
            tasks: Vec::default(),
            events_tx,
            events_rx,
            footer_tx,
        }
    }

    pub fn start(
        &mut self,
        client: &KubernetesClient,
        resource: ResourceRef,
        port: u16,
        address: SocketAddr,
    ) -> Result<(), PortForwardError> {
        if resource.kind.name() != PODS || resource.name.is_none() {
            return Err(PortForwardError::UnsupportedResource);
        }

        self.footer_tx
            .send(FooterMessage::info(
                format!(
                    "Port forward for {}:  {} -> {}",
                    resource.name.as_deref().unwrap_or_default(),
                    address,
                    port
                ),
                10_000,
            ))
            .unwrap();

        let pods: Api<Pod> = Api::namespaced(client.get_client(), resource.namespace.as_str());

        let mut task = PortForwardTask::new(self.events_tx.clone(), self.footer_tx.clone());
        task.run(pods, resource.name.unwrap_or_default(), port, address)?;

        self.tasks.push(task);

        Ok(())
    }

    /// Tries to get next [`PortForwardEvent`].
    pub fn try_next(&mut self) -> Option<PortForwardEvent> {
        self.events_rx.try_recv().ok()
    }

    /// Drains waiting [`PortForwardEvent`]s.
    pub fn drain(&mut self) {
        while self.events_rx.try_recv().is_ok() {}
    }
}

pub struct PortForwardTask {
    task: Option<JoinHandle<()>>,
    cancellation_token: Option<CancellationToken>,
    statistics: TaskStatistics,
    events_tx: UnboundedSender<PortForwardEvent>,
    footer_tx: UnboundedSender<FooterMessage>,
}

impl PortForwardTask {
    fn new(events_tx: UnboundedSender<PortForwardEvent>, footer_tx: UnboundedSender<FooterMessage>) -> Self {
        let statistics = TaskStatistics {
            active_connections: Arc::new(AtomicI16::new(0)),
            overall_connections: Arc::new(AtomicI32::new(0)),
            connection_errors: Arc::new(AtomicI32::new(0)),
        };

        Self {
            task: None,
            cancellation_token: None,
            statistics,
            events_tx,
            footer_tx,
        }
    }

    fn run(&mut self, pods: Api<Pod>, resource: String, port: u16, address: SocketAddr) -> Result<(), PortForwardError> {
        let cancellation_token = CancellationToken::new();
        let _cancellation_token = cancellation_token.clone();
        let _events_tx = self.events_tx.clone();
        let _footer_tx = self.footer_tx.clone();
        let _statistics = self.statistics.clone();

        let task = tokio::spawn(async move {
            _events_tx.send(PortForwardEvent::TaskStarted).unwrap();
            if let Ok(listener) = TcpListener::bind(address).await {
                while !_cancellation_token.is_cancelled() {
                    tokio::select! {
                        () = _cancellation_token.cancelled() => (),
                        result = listener.accept() => {
                            match result {
                                Ok((stream, _)) => {
                                    accept_connection(
                                        &pods,
                                        &resource,
                                        port,
                                        stream,
                                        _events_tx.clone(),
                                        _statistics.clone(),
                                    )
                                    .await
                                },
                                Err(e) => accept_error(e, &_events_tx, &_footer_tx, &_statistics.connection_errors),
                            };
                        }
                    }
                }
            }

            _events_tx.send(PortForwardEvent::TaskStopped).unwrap();
        });

        self.task = Some(task);
        self.cancellation_token = Some(cancellation_token);

        Ok(())
    }
}

#[derive(Clone)]
struct TaskStatistics {
    active_connections: Arc<AtomicI16>,
    overall_connections: Arc<AtomicI32>,
    connection_errors: Arc<AtomicI32>,
}

fn accept_error(
    error: std::io::Error,
    events_tx: &UnboundedSender<PortForwardEvent>,
    footer_tx: &UnboundedSender<FooterMessage>,
    connection_errors: &Arc<AtomicI32>,
) {
    let msg = format!("error accepting port forward connection: {}", error);

    warn!(msg);
    footer_tx.send(FooterMessage::error(msg, 0)).unwrap();

    connection_errors.fetch_add(1, Ordering::Relaxed);
    events_tx.send(PortForwardEvent::ConnectionError).unwrap();
}

async fn accept_connection(
    api: &Api<Pod>,
    pod_name: &str,
    port: u16,
    client_conn: tokio::net::TcpStream,
    events_tx: UnboundedSender<PortForwardEvent>,
    statistics: TaskStatistics,
) {
    let api = api.clone();
    let pod_name = pod_name.to_owned();
    tokio::spawn(async move {
        statistics.overall_connections.fetch_add(1, Ordering::Relaxed);
        statistics.active_connections.fetch_add(1, Ordering::Relaxed);
        events_tx.send(PortForwardEvent::ConnectionAccepted).unwrap();

        if let Err(e) = forward_connection(&api, &pod_name, port, client_conn).await {
            warn!("failed to forward connection: {}", e);
            statistics.connection_errors.fetch_add(1, Ordering::Relaxed);
        }

        statistics.active_connections.fetch_sub(1, Ordering::Relaxed);
        events_tx.send(PortForwardEvent::ConnectionClosed).unwrap();
    });
}

async fn forward_connection(
    api: &Api<Pod>,
    pod_name: &str,
    port: u16,
    mut client_conn: tokio::net::TcpStream,
) -> Result<(), PortForwardError> {
    let mut forwarder = api.portforward(pod_name, &[port]).await?;
    if let Some(mut upstream_conn) = forwarder.take_stream(port) {
        tokio::io::copy_bidirectional(&mut client_conn, &mut upstream_conn).await?;

        drop(upstream_conn);
        match forwarder.join().await {
            Ok(_) => Ok(()),
            Err(e) => Err(PortForwardError::PortforwardError(e.to_string())),
        }
    } else {
        Err(PortForwardError::PortNotFound)
    }
}
