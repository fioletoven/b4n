use futures::{SinkExt, channel::mpsc::Sender};
use k8s_openapi::api::core::v1::Pod;
use kube::{
    Api,
    api::{AttachParams, TerminalSize},
};
use std::sync::{Arc, RwLock};
use thiserror;
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;
use tracing::warn;
use tui_term::vt100::{self};

use crate::{
    app::utils::wait_for_task,
    kubernetes::{PodRef, client::KubernetesClient},
};

/// Possible errors from [`ShellBridge`].
#[derive(thiserror::Error, Debug)]
pub enum ShellBridgeError {
    /// Kubernetes client error.
    #[error("kubernetes client error")]
    KubeClientError(#[from] kube::Error),
}

/// Bridge between pod's shell and `b4n`'s TUI.
pub struct ShellBridge {
    task: Option<JoinHandle<()>>,
    cancellation_token: Option<CancellationToken>,
    input_tx: Option<UnboundedSender<Vec<u8>>>,
    size_tx: Option<UnboundedSender<TerminalSize>>,
    parser: Arc<RwLock<vt100::Parser>>,
    was_started: bool,
}

impl ShellBridge {
    /// Creates new [`ShellBridge`] instance.
    pub fn new(parser: Arc<RwLock<vt100::Parser>>) -> Self {
        Self {
            task: None,
            cancellation_token: None,
            input_tx: None,
            size_tx: None,
            parser,
            was_started: false,
        }
    }

    /// Starts new shell process.
    pub fn start(&mut self, client: &KubernetesClient, pod: PodRef) -> Result<(), ShellBridgeError> {
        let cancellation_token = CancellationToken::new();
        let _cancellation_token = cancellation_token.clone();
        let _client = client.get_client();
        let _parser = self.parser.clone();

        let (input_tx, _input_rx) = mpsc::unbounded_channel();
        self.input_tx = Some(input_tx);

        let (size_tx, mut _size_rx) = mpsc::unbounded_channel();
        self.size_tx = Some(size_tx);

        let task = tokio::spawn(async move {
            let api: Api<Pod> = Api::namespaced(_client, pod.namespace.as_str());
            let attach_params = AttachParams::interactive_tty();

            let mut attached = match api.exec(&pod.name, vec!["sh"], &attach_params).await {
                Ok(attached) => attached,
                Err(err) => {
                    warn!("Cannot attach to the pod's shell: {}", err);
                    return;
                },
            };

            let stdin = attached.stdin().unwrap();
            let stdout = attached.stdout().unwrap();
            let sizer = attached.terminal_size().unwrap();

            tokio::select! {
                _ = tokio::spawn(input_bridge(stdin, _input_rx, _cancellation_token.clone())) => (),
                _ = tokio::spawn(output_bridge(stdout, _parser, _cancellation_token.clone())) => (),
                _ = tokio::spawn(sizer_bridge(sizer, _size_rx, _cancellation_token.clone())) => (),
            }
        });

        self.cancellation_token = Some(cancellation_token);
        self.task = Some(task);
        self.was_started = true;

        Ok(())
    }

    /// Cancels [`ShellBridge`] task.
    pub fn cancel(&mut self) {
        if let Some(cancellation_token) = self.cancellation_token.take() {
            cancellation_token.cancel();
        }
    }

    /// Cancels [`ShellBridge`] task and waits for it to finish.
    pub fn stop(&mut self) {
        self.cancel();
        wait_for_task(self.task.take(), "IO bridge");
    }

    /// Sends user input bytes to the attached process.
    pub fn send(&self, data: Vec<u8>) {
        if self.is_running() {
            if let Some(tx) = &self.input_tx {
                tx.send(data).unwrap();
            }
        }
    }

    /// Sets size of the bridged terminal.
    pub fn set_terminal_size(&mut self, width: u16, height: u16) {
        if self.is_running() {
            if let Some(tx) = &self.size_tx {
                tx.send(TerminalSize { width, height }).unwrap();
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

impl Drop for ShellBridge {
    fn drop(&mut self) {
        self.cancel();
    }
}

async fn input_bridge(
    mut stdin: impl AsyncWrite + Unpin,
    mut input_rx: UnboundedReceiver<Vec<u8>>,
    cancellation_token: CancellationToken,
) {
    while !cancellation_token.is_cancelled() {
        tokio::select! {
            _ = cancellation_token.cancelled() => (),
            Some(input) = input_rx.recv() => {
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
    }
}

async fn output_bridge(
    mut stdout: impl AsyncRead + Unpin,
    parser: Arc<RwLock<vt100::Parser>>,
    cancellation_token: CancellationToken,
) {
    let mut buf = [0u8; 8192];
    let mut processed_buf = Vec::new();

    while !cancellation_token.is_cancelled() {
        tokio::select! {
            _ = cancellation_token.cancelled() => (),
            Ok(size) = stdout.read(&mut buf) => {
                if size == 0 {
                    break;
                }
                if size > 0 {
                    processed_buf.extend_from_slice(&buf[..size]);
                    let mut parser = parser.write().unwrap();
                    parser.process(&processed_buf);
                    processed_buf.clear();
                }
            }
        }
    }
}

async fn sizer_bridge(
    mut sizer: Sender<TerminalSize>,
    mut size_rx: UnboundedReceiver<TerminalSize>,
    cancellation_token: CancellationToken,
) {
    while !cancellation_token.is_cancelled() {
        tokio::select! {
            _ = cancellation_token.cancelled() => (),
            Some(size) = size_rx.recv() => {
                if let Err(err) = sizer.send(size).await {
                    warn!("Cannot set attached process terminal size: {}", err);
                }
            },
        }
    }
}
