use k8s_openapi::api::core::v1::Pod;
use kube::Api;
use std::net::SocketAddr;
use tokio::{net::TcpListener, sync::mpsc::UnboundedSender, task::JoinHandle};
use tokio_util::sync::CancellationToken;

use crate::{
    kubernetes::{ResourceRef, client::KubernetesClient, resources::PODS},
    ui::widgets::FooterMessage,
};

/// Possible errors from [`PortForwarder`].
#[derive(thiserror::Error, Debug)]
pub enum PortForwardError {
    /// Provided resource is not a `pods` kind or has no name.
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
    #[error("portforward task error")]
    PortforwardError,
}

pub struct PortForwardTask {
    task: Option<JoinHandle<()>>,
    cancellation_token: Option<CancellationToken>,
    footer_tx: UnboundedSender<FooterMessage>,
}

impl PortForwardTask {
    fn new(footer_tx: UnboundedSender<FooterMessage>) -> Self {
        Self {
            task: None,
            cancellation_token: None,
            footer_tx,
        }
    }

    fn run(&mut self, pods: Api<Pod>, resource: String, port: u16, address: SocketAddr) -> Result<(), PortForwardError> {
        let cancellation_token = CancellationToken::new();
        let _cancellation_token = cancellation_token.clone();

        let task = tokio::spawn(async move {
            if let Ok(listener) = TcpListener::bind(address).await {
                while !_cancellation_token.is_cancelled() {
                    tokio::select! {
                        () = _cancellation_token.cancelled() => (),
                        result = listener.accept() => {
                            match result {
                                Ok((stream, _)) => new_connection(&pods, &resource, port, stream).await,
                                Err(e) => tracing::error!("error accepting connection: {}", e),
                            };
                        }
                    }
                }
            }
        });

        self.task = Some(task);
        self.cancellation_token = Some(cancellation_token);

        Ok(())
    }
}

async fn new_connection(api: &Api<Pod>, pod_name: &str, port: u16, client_conn: tokio::net::TcpStream) {
    let api = api.clone();
    let pod_name = pod_name.to_owned();
    tokio::spawn(async move {
        if let Err(e) = forward_connection(&api, &pod_name, port, client_conn).await {
            tracing::error!("failed to forward connection: {}", e);
        }
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
        if forwarder.join().await.is_err() {
            return Err(PortForwardError::PortforwardError);
        }

        Ok(())
    } else {
        Err(PortForwardError::PortNotFound)
    }
}

pub struct PortForwarder {
    tasks: Vec<PortForwardTask>,
    footer_tx: UnboundedSender<FooterMessage>,
}

impl PortForwarder {
    pub fn new(footer_tx: UnboundedSender<FooterMessage>) -> Self {
        Self {
            tasks: Vec::default(),
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

        let _ = self.footer_tx.send(FooterMessage::info(
            format!(
                "Port forward for {}:  {} -> {}",
                resource.name.as_deref().unwrap_or_default(),
                address,
                port
            ),
            10_000,
        ));

        let pods: Api<Pod> = Api::namespaced(client.get_client(), resource.namespace.as_str());

        let mut task = PortForwardTask::new(self.footer_tx.clone());
        task.run(pods, resource.name.unwrap_or_default(), port, address)?;

        self.tasks.push(task);

        Ok(())
    }
}
