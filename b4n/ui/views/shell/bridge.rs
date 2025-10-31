use b4n_kube::PodRef;
use futures::{SinkExt, channel::mpsc::Sender};
use k8s_openapi::api::core::v1::Pod;
use kube::{
    Api, Client,
    api::{AttachParams, TerminalSize},
};
use std::sync::{
    Arc, RwLock,
    atomic::{AtomicBool, Ordering},
};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    runtime::Handle,
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;
use tracing::warn;
use tui_term::vt100::{self};

/// Bridge between pod's shell and `b4n`'s TUI.
pub struct ShellBridge {
    runtime: Handle,
    task: Option<JoinHandle<()>>,
    cancellation_token: Option<CancellationToken>,
    input_tx: Option<UnboundedSender<Vec<u8>>>,
    size_tx: Option<UnboundedSender<TerminalSize>>,
    parser: Arc<RwLock<vt100::Parser>>,
    has_error: Arc<AtomicBool>,
    is_running: Arc<AtomicBool>,
    was_started: bool,
    shell: Option<String>,
}

impl ShellBridge {
    /// Creates new [`ShellBridge`] instance.
    pub fn new(runtime: Handle, parser: Arc<RwLock<vt100::Parser>>) -> Self {
        Self {
            runtime,
            task: None,
            cancellation_token: None,
            input_tx: None,
            size_tx: None,
            parser,
            has_error: Arc::new(AtomicBool::new(false)),
            is_running: Arc::new(AtomicBool::new(false)),
            was_started: false,
            shell: None,
        }
    }

    /// Starts new shell process.\
    /// **Note** that it stops the old task if it is running.
    pub fn start(&mut self, client: Client, pod: PodRef, shell: impl Into<String>) {
        self.stop();

        let cancellation_token = CancellationToken::new();
        let _cancellation_token = cancellation_token.clone();
        let _parser = self.parser.clone();

        let (input_tx, _input_rx) = mpsc::unbounded_channel();
        self.input_tx = Some(input_tx);

        let (size_tx, _size_rx) = mpsc::unbounded_channel();
        self.size_tx = Some(size_tx);

        let _shell = shell.into();
        self.shell = Some(_shell.clone());

        self.has_error.store(false, Ordering::Relaxed);
        let _has_error = Arc::clone(&self.has_error);
        let _is_running = Arc::clone(&self.is_running);

        let task = self.runtime.spawn(async move {
            let api: Api<Pod> = Api::namespaced(client, pod.namespace.as_str());
            let attach_params = AttachParams::interactive_tty();

            let mut attached = match api.exec(&pod.name, vec![_shell], &attach_params).await {
                Ok(attached) => attached,
                Err(err) => {
                    warn!("Cannot attach to the pod's shell: {}", err);
                    _has_error.store(true, Ordering::Relaxed);
                    return;
                },
            };

            let stdin = attached.stdin().unwrap();
            let stdout = attached.stdout().unwrap();
            let tty_resize = attached.terminal_size().unwrap();

            _is_running.store(true, Ordering::Relaxed);

            let ((), output_closed_too_soon, ()) = tokio::join! {
                input_bridge(stdin, _input_rx, _cancellation_token.clone()),
                output_bridge(stdout, _parser, _cancellation_token.clone()),
                resize_bridge(tty_resize, _size_rx, _cancellation_token.clone())
            };

            _is_running.store(false, Ordering::Relaxed);
            _has_error.store(output_closed_too_soon, Ordering::Relaxed);
        });

        self.cancellation_token = Some(cancellation_token);
        self.task = Some(task);
        self.was_started = true;
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
        b4n_common::tasks::wait_for_task(self.task.take(), "shell bridge");
    }

    /// Sends user input bytes to the attached process.
    pub fn send(&self, data: Vec<u8>) {
        if self.is_running()
            && let Some(tx) = &self.input_tx
            && let Err(err) = tx.send(data)
        {
            warn!("Cannot send data to the attached process: {}", err);
        }
    }

    /// Sets size of the bridged terminal.
    pub fn set_terminal_size(&mut self, width: u16, height: u16) {
        if self.is_running()
            && let Some(tx) = &self.size_tx
        {
            let _ = tx.send(TerminalSize { width, height });
        }
    }

    /// Returns name of the shell that this bridge is/was attached to.
    pub fn shell(&self) -> Option<&str> {
        self.shell.as_deref()
    }

    /// Returns `true` if attached process is running.
    pub fn is_running(&self) -> bool {
        self.task.as_ref().is_some_and(|t| !t.is_finished()) && self.is_running.load(Ordering::Relaxed)
    }

    /// Returns `true` if attached process has finished.
    pub fn is_finished(&self) -> bool {
        (self.was_started && self.task.is_none()) || self.task.as_ref().is_some_and(JoinHandle::is_finished)
    }

    /// Returns `true` if attached process has/had an error state.
    pub fn has_error(&self) -> bool {
        self.has_error.load(Ordering::Relaxed)
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
            () = cancellation_token.cancelled() => (),
            Some(input) = input_rx.recv() => {
                if let Err(err) = stdin.write_all(&input[..]).await {
                    warn!("Cannot write to the attached process stdin: {}", err);
                    cancellation_token.cancel();
                    return;
                }
                if let Err(err) = stdin.flush().await {
                    warn!("Cannot flush the attached process stdin: {}", err);
                    cancellation_token.cancel();
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
) -> bool {
    let mut buf = [0u8; 8192];
    let mut processed_buf = Vec::new();
    let mut total_bytes_read = 0;

    while !cancellation_token.is_cancelled() {
        tokio::select! {
            () = cancellation_token.cancelled() => (),
            Ok(size) = stdout.read(&mut buf) => {
                if size == 0 {
                    cancellation_token.cancel();
                    return total_bytes_read == 0;
                }
                if size > 0 {
                    processed_buf.extend_from_slice(&buf[..size]);
                    let mut parser = parser.write().unwrap();
                    parser.process(&processed_buf);
                    processed_buf.clear();
                    total_bytes_read += size;
                }
            }
        }
    }

    false
}

async fn resize_bridge(
    mut sender: Sender<TerminalSize>,
    mut receiver: UnboundedReceiver<TerminalSize>,
    cancellation_token: CancellationToken,
) {
    while !cancellation_token.is_cancelled() {
        tokio::select! {
            () = cancellation_token.cancelled() => (),
            Some(size) = receiver.recv() => {
                if let Err(err) = sender.send(size).await {
                    warn!("Cannot resize the attached process tty: {}", err);
                }
            },
        }
    }
}
