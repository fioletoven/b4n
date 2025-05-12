use k8s_openapi::api::core::v1::Pod;
use kube::{Api, api::AttachParams};
use std::sync::{Arc, RwLock};
use thiserror;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::mpsc::{self, UnboundedSender},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;
use tracing::warn;
use tui_term::vt100::{self};

use crate::{
    app::utils::wait_for_task,
    kubernetes::{PodRef, client::KubernetesClient},
};

/// Possible errors from [`IOBridge`].
#[derive(thiserror::Error, Debug)]
pub enum IOBridgeError {
    /// Kubernetes client error.
    #[error("kubernetes client error")]
    KubeClientError(#[from] kube::Error),
}

/// Bridge between pod process and b4n TUI.
pub struct IOBridge {
    task: Option<JoinHandle<()>>,
    cancellation_token: Option<CancellationToken>,
    input_tx: Option<UnboundedSender<Vec<u8>>>,
    parser: Arc<RwLock<vt100::Parser>>,
    was_started: bool,
}

impl IOBridge {
    pub fn new(parser: Arc<RwLock<vt100::Parser>>) -> Self {
        Self {
            task: None,
            cancellation_token: None,
            input_tx: None,
            parser,
            was_started: false,
        }
    }

    pub fn start(&mut self, client: &KubernetesClient, pod: PodRef) -> Result<(), IOBridgeError> {
        let cancellation_token = CancellationToken::new();
        let _cancellation_token = cancellation_token.clone();
        let _client = client.get_client();
        let _parser = self.parser.clone();

        let (input_tx, mut _input_rx) = mpsc::unbounded_channel();
        self.input_tx = Some(input_tx);

        let task = tokio::spawn(async move {
            let api: Api<Pod> = Api::namespaced(_client, pod.namespace.as_str());
            let attach_params = AttachParams::interactive_tty();
            let mut attached = match api.exec(&pod.name, vec!["/bin/sh"], &attach_params).await {
                Ok(attached) => attached,
                Err(err) => {
                    warn!("Cannot attach to the pod's shell: {}", err);
                    return;
                },
            };

            let input_task = tokio::spawn({
                let mut stdin = attached.stdin().unwrap();
                async move {
                    while let Some(input) = _input_rx.recv().await {
                        if let Err(err) = stdin.write_all(&input[..]).await {
                            warn!("Cannot write to the attached process stdin: {}", err);
                            return;
                        }
                        if let Err(err) = stdin.flush().await {
                            warn!("Cannot flush the attached process stdin: {}", err);
                            return;
                        }
                    }
                }
            });

            let output_task = tokio::spawn({
                let mut stdout = attached.stdout().unwrap();
                async move {
                    let mut buf = [0u8; 8192];
                    let mut processed_buf = Vec::new();
                    while let Ok(size) = stdout.read(&mut buf).await {
                        if size == 0 {
                            break;
                        }
                        if size > 0 {
                            processed_buf.extend_from_slice(&buf[..size]);
                            let mut parser = _parser.write().unwrap();
                            parser.process(&processed_buf);
                            processed_buf.clear();
                        }
                    }
                }
            });

            tokio::select! {
                _ = _cancellation_token.cancelled() => (),
                _ = input_task => (),
                _ = output_task => (),
            }
        });

        self.cancellation_token = Some(cancellation_token);
        self.task = Some(task);
        self.was_started = true;

        Ok(())
    }

    /// Cancels [`IOBridge`] task.
    pub fn cancel(&mut self) {
        if let Some(cancellation_token) = self.cancellation_token.take() {
            cancellation_token.cancel();
        }
    }

    /// Cancels [`IOBridge`] task and waits for it to finish.
    pub fn stop(&mut self) {
        self.cancel();
        wait_for_task(self.task.take(), "IO bridge");
    }

    /// Sends user input bytes to the attached process.
    pub fn send(&self, data: Vec<u8>) {
        if self.is_running() {
            if let Some(tx) = &self.input_tx {
                tx.send(data).unwrap()
            }
        }
    }

    /// Returns `true` if attached process is running.
    pub fn is_running(&self) -> bool {
        self.task.as_ref().is_some_and(|t| !t.is_finished())
    }

    /// Returns `true` if attached process has finished.
    pub fn is_finished(&self) -> bool {
        (self.was_started && self.task.is_none()) || self.task.as_ref().is_some_and(|t| t.is_finished())
    }
}

impl Drop for IOBridge {
    fn drop(&mut self) {
        self.cancel();
    }
}
