use b4n_kube::ContainerRef;
use futures::{SinkExt, channel::mpsc::Sender};
use k8s_openapi::api::core::v1::Pod;
use kube::api::{AttachParams, TerminalSize};
use kube::{Api, Client};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, RwLock};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::runtime::Handle;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tui_term::vt100::{self};

/// Bridge between pod's shell and `b4n`'s TUI.
pub struct ShellBridge {
    runtime: Handle,
    task: Option<JoinHandle<()>>,
    cancellation_token: Option<CancellationToken>,
    input_tx: Option<UnboundedSender<Vec<u8>>>,
    size_tx: Option<UnboundedSender<TerminalSize>>,
    parser: Arc<RwLock<vt100::Parser>>,
    is_attach: bool,
    is_running: Arc<AtomicBool>,
    has_error: Arc<AtomicBool>,
    was_started: bool,
    shell: Option<String>,
    application_mode: Arc<AtomicU8>,
    mouse_mode: Arc<AtomicU8>,
}

impl ShellBridge {
    /// Creates new [`ShellBridge`] instance.
    pub fn new(runtime: Handle, parser: Arc<RwLock<vt100::Parser>>, is_attach: bool) -> Self {
        Self {
            runtime,
            task: None,
            cancellation_token: None,
            input_tx: None,
            size_tx: None,
            parser,
            is_attach,
            is_running: Arc::new(AtomicBool::new(false)),
            has_error: Arc::new(AtomicBool::new(false)),
            was_started: false,
            shell: None,
            application_mode: Arc::new(AtomicU8::new(0)),
            mouse_mode: Arc::new(AtomicU8::new(0)),
        }
    }

    /// Starts new shell process.\
    /// **Note** that it stops the old task if it is running.
    pub fn start(&mut self, client: Client, pod: ContainerRef, shell: impl Into<String>, size: TerminalSize) {
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
        let _is_attach = self.is_attach;
        let _application_mode = Arc::clone(&self.application_mode);
        let _mouse_mode = Arc::clone(&self.mouse_mode);

        let task = self.runtime.spawn(async move {
            let api: Api<Pod> = Api::namespaced(client, pod.namespace.as_str());
            let mut attach_params = AttachParams::interactive_tty();
            if let Some(container) = pod.container {
                attach_params = attach_params.container(container.clone());
            }

            let attach_result = if _is_attach {
                api.attach(&pod.name, &attach_params).await
            } else {
                api.exec(&pod.name, vec![_shell], &attach_params).await
            };

            let mut attached = match attach_result {
                Ok(attached) => attached,
                Err(err) => {
                    if _is_attach {
                        tracing::warn!("Cannot attach to the pod's main process: {}", err);
                    } else {
                        tracing::warn!("Cannot attach to the pod's shell: {}", err);
                    }
                    _has_error.store(true, Ordering::Relaxed);
                    return;
                },
            };

            let Some(mut stdin) = attached.stdin() else {
                let name = attach_params.container.as_deref().unwrap_or("unknown");
                tracing::warn!("Unable to use an stdin for container '{}'", name);
                _has_error.store(true, Ordering::Relaxed);
                return;
            };
            let Some(stdout) = attached.stdout() else {
                let name = attach_params.container.as_deref().unwrap_or("unknown");
                tracing::warn!("Unable to use an stdout for container '{}'", name);
                _has_error.store(true, Ordering::Relaxed);
                return;
            };
            let Some(tty_resize) = attached.terminal_size() else {
                let name = attach_params.container.as_deref().unwrap_or("unknown");
                tracing::warn!("Unable to use a TTY for container '{}'", name);
                _has_error.store(true, Ordering::Relaxed);
                return;
            };

            if _is_attach {
                _is_running.store(true, Ordering::Relaxed);

                // DSR query terminal mode
                let query = vec![27, 91, 63, 49, 36, 112];
                let _ = stdin.write_all(&query).await;
                let _ = stdin.flush().await;
            }

            let ((), output_closed_too_soon, ()) = tokio::join! {
                input_bridge(stdin, _input_rx, _cancellation_token.clone()),
                output_bridge(stdout, _parser, _cancellation_token.clone(), Arc::clone(&_is_running), _application_mode, _mouse_mode),
                resize_bridge(tty_resize, _size_rx, _cancellation_token.clone())
            };

            _is_running.store(false, Ordering::Relaxed);
            _has_error.store(output_closed_too_soon, Ordering::Relaxed);
        });

        if let Some(tx) = &self.size_tx {
            let _ = tx.send(size);
        }

        self.cancellation_token = Some(cancellation_token);
        self.task = Some(task);
        self.was_started = true;
    }

    /// Cancels [`ShellBridge`] task.
    pub fn cancel(&mut self) {
        if let Some(cancellation_token) = self.cancellation_token.take() {
            cancellation_token.cancel();
            self.is_running.store(false, Ordering::Relaxed);
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
            tracing::warn!("Cannot send data to the attached process: {}", err);
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
        self.was_started && self.task.as_ref().is_some_and(|t| !t.is_finished()) && self.is_running.load(Ordering::Relaxed)
    }

    /// Returns `true` if attached process has finished.
    pub fn is_finished(&self) -> bool {
        (self.was_started && self.task.is_none()) || self.task.as_ref().is_some_and(JoinHandle::is_finished)
    }

    /// Returns `true` if attached process has/had an error state.
    pub fn has_error(&self) -> bool {
        self.has_error.load(Ordering::Relaxed)
    }

    /// Returns `true` if terminal is in application mode.
    pub fn is_application_mode(&self) -> Option<bool> {
        match self.application_mode.load(Ordering::Relaxed) {
            0 => None,
            1 => Some(false),
            _ => Some(true),
        }
    }

    /// Returns `true` if terminal has mouse enabled.
    pub fn is_mouse_enabled(&self) -> Option<bool> {
        match self.mouse_mode.load(Ordering::Relaxed) {
            0 => None,
            1 => Some(false),
            _ => Some(true),
        }
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
                    tracing::warn!("Cannot write to the attached process stdin: {}", err);
                    cancellation_token.cancel();
                    return;
                }
                if let Err(err) = stdin.flush().await {
                    tracing::warn!("Cannot flush the attached process stdin: {}", err);
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
    is_running: Arc<AtomicBool>,
    cursor_key_mode: Arc<AtomicU8>,
    mouse_mode: Arc<AtomicU8>,
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

                is_running.store(true, Ordering::Relaxed);

                processed_buf.extend_from_slice(&buf[..size]);

                let (app_mode_enabled, mouse_enabled) = detect_terminal_modes(&processed_buf);
                if let Some(is_enabled) = app_mode_enabled {
                    cursor_key_mode.store(if is_enabled { 2 } else { 1 }, Ordering::Relaxed);
                }
                if let Some(is_enabled) = mouse_enabled {
                    mouse_mode.store(if is_enabled { 2 } else { 1 }, Ordering::Relaxed);
                }

                let mut parser = parser.write().unwrap();
                parser.process(&processed_buf);
                processed_buf.clear();
                total_bytes_read += size;
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
                    tracing::warn!("Cannot resize the attached process tty: {}", err);
                }
            },
        }
    }
}

fn detect_terminal_modes(data: &[u8]) -> (Option<bool>, Option<bool>) {
    const CSI_PREFIX: &[u8] = &[27, 91, 63]; // ESC [ ?

    let mut application_mode = None;
    let mut mouse_mode = None;
    let mut i = 0;

    while i < data.len() {
        if i + CSI_PREFIX.len() > data.len() {
            break;
        }

        if &data[i..i + CSI_PREFIX.len()] != CSI_PREFIX {
            i += 1;
            continue;
        }

        let mut end = i + CSI_PREFIX.len();
        let mut terminator = None;

        // Find end of the escape sequence, example: `ESC [ ? 1006 ; 1000 h`.
        while end < data.len() {
            let byte = data[end];
            if byte == b'h' || byte == b'l' {
                terminator = Some(byte);
                break;
            }

            if !byte.is_ascii_digit() && byte != b';' {
                break;
            }

            end += 1;
        }

        let Some(terminator) = terminator else { break };
        let params = &data[i + CSI_PREFIX.len()..end];

        let mut param_start = 0;
        for (j, &byte) in params.iter().enumerate() {
            if byte == b';' || j == params.len() - 1 {
                let param_end = if byte == b';' { j } else { j + 1 };
                let param = &params[param_start..param_end];

                match param {
                    b"1" => application_mode = Some(terminator == b'h'),
                    b"1000" | b"1002" | b"1003" | b"1006" => mouse_mode = Some(terminator == b'h'),
                    _ => {},
                }

                param_start = param_end + 1;
            }
        }

        i = end + 1;
    }

    (application_mode, mouse_mode)
}
