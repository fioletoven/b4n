use k8s_openapi::{api::core::v1::Pod, apimachinery::pkg::apis::meta::v1::Time, chrono::Utc};
use kube::Api;
use std::{
    error::Error,
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
use uuid::Uuid;

use crate::{
    core::utils::wait_for_task,
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

/// Holds all port forwarding tasks for the current context.
pub struct PortForwarder {
    tasks: Vec<PortForwardTask>,
    events_tx: UnboundedSender<PortForwardEvent>,
    events_rx: UnboundedReceiver<PortForwardEvent>,
    footer_tx: UnboundedSender<FooterMessage>,
}

impl PortForwarder {
    /// Creates new [`PortForwarder`] instance.
    pub fn new(footer_tx: UnboundedSender<FooterMessage>) -> Self {
        let (events_tx, events_rx) = mpsc::unbounded_channel();
        Self {
            tasks: Vec::default(),
            events_tx,
            events_rx,
            footer_tx,
        }
    }

    /// Returns port forward tasks list.
    pub fn tasks(&self) -> &[PortForwardTask] {
        &self.tasks
    }

    /// Removes completed port forward tasks.
    pub fn cleanup_tasks(&mut self) {
        self.tasks.retain(|t| t.task.as_ref().is_none_or(|t| !t.is_finished()));
    }

    /// Starts port forwarding task.
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
        task.run(pods, resource, port, address)?;

        self.tasks.push(task);

        Ok(())
    }

    /// Stops port forwarding task with the specified `uuid`.
    pub fn stop(&mut self, uuid: &str) {
        if let Some(index) = self.tasks.iter().position(|t| t.uuid == uuid) {
            let _ = self.tasks.swap_remove(index);
        }
    }

    /// Cancels all [`PortForwarder`] tasks.
    pub fn cancel_all(&mut self) {
        for task in &mut self.tasks {
            task.cancel();
        }
    }

    /// Cancels all tasks running in [`PortForwarder`] instance.
    pub fn stop_all(&mut self) {
        for task in &mut self.tasks {
            task.stop();
        }

        self.tasks.clear();
        self.drain();
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

impl Drop for PortForwarder {
    fn drop(&mut self) {
        self.cancel_all();
    }
}

/// Task that handles port forwarding for the specific pod port.
pub struct PortForwardTask {
    pub uuid: String,
    pub resource: ResourceRef,
    pub bind_address: String,
    pub port: u16,
    pub start_time: Option<Time>,
    pub statistics: TaskStatistics,
    task: Option<JoinHandle<()>>,
    cancellation_token: Option<CancellationToken>,
    events_tx: UnboundedSender<PortForwardEvent>,
    footer_tx: UnboundedSender<FooterMessage>,
}

impl PortForwardTask {
    /// Creates new [`PortForwardTask`] instance.
    fn new(events_tx: UnboundedSender<PortForwardEvent>, footer_tx: UnboundedSender<FooterMessage>) -> Self {
        let statistics = TaskStatistics {
            active_connections: Arc::new(AtomicI16::new(0)),
            overall_connections: Arc::new(AtomicI32::new(0)),
            connection_errors: Arc::new(AtomicI32::new(0)),
        };

        Self {
            uuid: Uuid::new_v4()
                .hyphenated()
                .encode_lower(&mut Uuid::encode_buffer())
                .to_owned(),
            resource: ResourceRef::default(),
            bind_address: String::default(),
            port: 0,
            start_time: None,
            statistics,
            task: None,
            cancellation_token: None,
            events_tx,
            footer_tx,
        }
    }

    /// Runs port forward task.
    fn run(&mut self, pods: Api<Pod>, resource: ResourceRef, port: u16, address: SocketAddr) -> Result<(), PortForwardError> {
        self.bind_address = address.to_string();
        self.port = port;

        let cancellation_token = CancellationToken::new();
        let _cancellation_token = cancellation_token.clone();
        let _pod_name = resource.name.as_deref().unwrap_or_default().to_owned();
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
                                        &_pod_name,
                                        port,
                                        stream,
                                        _events_tx.clone(),
                                        _statistics.clone(),
                                        _cancellation_token.clone(),
                                    )
                                    .await;
                                },
                                Err(e) => accept_error(&e, &_events_tx, &_footer_tx, &_statistics.connection_errors),
                            }
                        }
                    }
                }
            }

            _events_tx.send(PortForwardEvent::TaskStopped).unwrap();
        });

        self.task = Some(task);
        self.cancellation_token = Some(cancellation_token);
        self.resource = resource;
        self.start_time = Some(Time(Utc::now()));

        Ok(())
    }

    /// Cancels [`PortForwardTask`] task.
    fn cancel(&mut self) {
        if let Some(cancellation_token) = self.cancellation_token.take() {
            cancellation_token.cancel();
        }
    }

    /// Cancels [`PortForwardTask`] task and waits until it is finished.
    fn stop(&mut self) {
        self.cancel();
        wait_for_task(self.task.take(), "port forward");
    }
}

impl Drop for PortForwardTask {
    fn drop(&mut self) {
        self.cancel();
    }
}

#[derive(Clone)]
pub struct TaskStatistics {
    pub active_connections: Arc<AtomicI16>,
    pub overall_connections: Arc<AtomicI32>,
    pub connection_errors: Arc<AtomicI32>,
}

fn accept_error(
    error: &std::io::Error,
    events_tx: &UnboundedSender<PortForwardEvent>,
    footer_tx: &UnboundedSender<FooterMessage>,
    connection_errors: &Arc<AtomicI32>,
) {
    let msg = format!("error accepting port forward connection: {error}");

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
    cancellation_token: CancellationToken,
) {
    let api = api.clone();
    let pod_name = pod_name.to_owned();
    tokio::spawn(async move {
        statistics.overall_connections.fetch_add(1, Ordering::Relaxed);
        statistics.active_connections.fetch_add(1, Ordering::Relaxed);
        events_tx.send(PortForwardEvent::ConnectionAccepted).unwrap();

        if let Err(error) = forward_connection(&api, &pod_name, port, client_conn).await {
            warn!("failed to forward connection: {}", error);
            statistics.connection_errors.fetch_add(1, Ordering::Relaxed);

            match error {
                PortForwardError::KubeError(_) | PortForwardError::PortNotFound => {
                    cancellation_token.cancel();
                },
                _ => (),
            }
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
            Ok(()) => Ok(()),
            Err(error) => {
                if error
                    .source()
                    .is_some_and(|e| format!("{e:?}") == "Protocol(SendAfterClosing)")
                {
                    Ok(())
                } else {
                    Err(PortForwardError::PortforwardError(error.to_string()))
                }
            },
        }
    } else {
        Err(PortForwardError::PortNotFound)
    }
}
