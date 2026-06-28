use kube::api::TerminalSize;
use portable_pty::Child;
use ratatui::layout::Rect;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex, RwLock};
use tokio::runtime::Handle;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tui_term::vt100;

use crate::ui::views::shell::terminal::{TerminalState, handle_terminal_queries, update_terminal_state};

/// Bridge between external command and `b4n`'s TUI.
pub struct CmdBridge {
    runtime: Handle,
    task: Option<JoinHandle<()>>,
    cancellation_token: Option<CancellationToken>,
    input_tx: Option<UnboundedSender<Vec<u8>>>,
    size_tx: Option<UnboundedSender<(u16, u16)>>,
    parser: Arc<RwLock<vt100::Parser>>,
    state: TerminalState,
    was_started: bool,
    command: Option<String>,
}

impl CmdBridge {
    /// Creates new [`CmdBridge`] instance.
    pub fn new(runtime: Handle, area: Rect, scrollback_len: usize) -> Self {
        let parser = Arc::new(RwLock::new(vt100::Parser::new(area.height, area.width, scrollback_len)));
        Self {
            runtime,
            task: None,
            cancellation_token: None,
            input_tx: None,
            size_tx: None,
            parser,
            state: TerminalState::default(),
            was_started: false,
            command: None,
        }
    }

    /// Starts the external binary process.\
    /// **Note** that it stops the old task if it is running.
    pub fn start(&mut self, command: impl Into<String>, args: Vec<String>, size: TerminalSize) {
        self.stop();

        let cancellation_token = CancellationToken::new();
        let _cancellation_token = cancellation_token.clone();
        let _parser = self.parser.clone();

        let (input_tx, _input_rx) = mpsc::unbounded_channel::<Vec<u8>>();
        let _response_tx = input_tx.clone();
        self.input_tx = Some(input_tx);

        let (size_tx, _size_rx) = mpsc::unbounded_channel::<(u16, u16)>();
        self.size_tx = Some(size_tx);

        let _command = command.into();
        self.command = Some(_command.clone());

        self.state.set_error(false);
        let mut _state = self.state.clone();

        let task = self.runtime.spawn(async move {
            let pty_system = portable_pty::native_pty_system();
            let pty_size = portable_pty::PtySize {
                rows: size.height,
                cols: size.width,
                ..Default::default()
            };

            let pair = match pty_system.openpty(pty_size) {
                Ok(p) => p,
                Err(err) => {
                    tracing::warn!("Cannot open PTY: {}", err);
                    _state.set_error(true);
                    return;
                },
            };

            let mut cmd = portable_pty::CommandBuilder::new(&_command);
            cmd.args(&args);

            let child = match pair.slave.spawn_command(cmd) {
                Ok(c) => c,
                Err(err) => {
                    tracing::warn!("Cannot spawn command '{}': {}", _command, err);
                    _state.set_error(true);
                    return;
                },
            };

            drop(pair.slave);

            let writer = match pair.master.take_writer() {
                Ok(w) => w,
                Err(err) => {
                    tracing::warn!("Cannot get PTY writer for '{}': {}", _command, err);
                    _state.set_error(true);
                    return;
                },
            };

            let reader = match pair.master.try_clone_reader() {
                Ok(r) => r,
                Err(err) => {
                    tracing::warn!("Cannot get PTY reader for '{}': {}", _command, err);
                    _state.set_error(true);
                    return;
                },
            };

            _state.set_running(true);

            let master = Arc::new(Mutex::new(pair.master));
            let _master = Arc::clone(&master);

            let child_task = tokio::spawn({
                let _cancellation_token = _cancellation_token.clone();
                let _command = _command.clone();
                async move {
                    let ended_with_error = wait_for_child(&_command, child).await;
                    _cancellation_token.cancel();
                    ended_with_error
                }
            });

            let ((), output_closed_too_soon, ()) = tokio::join! {
                input_bridge(writer, _input_rx, _cancellation_token.clone()),
                output_bridge(
                    reader,
                    _parser,
                    _cancellation_token.clone(),
                    _response_tx,
                    _state.clone(),
                    TerminalSize { width: size.width, height: size.height },
                ),
                resize_bridge(_master, _size_rx, _cancellation_token.clone()),
            };

            _cancellation_token.cancel();

            let ended_with_error = child_task.await.unwrap_or(true);
            _state.set_error(ended_with_error || output_closed_too_soon);
            _state.set_running(false);

            drop(master);
        });

        self.cancellation_token = Some(cancellation_token);
        self.task = Some(task);
        self.was_started = true;
    }

    /// Cancels [`CmdBridge`] task.
    pub fn cancel(&mut self) {
        if let Some(token) = self.cancellation_token.take() {
            token.cancel();
            self.state.set_running(false);
        }
    }

    /// Cancels [`CmdBridge`] task and waits for it to finish.
    pub fn stop(&mut self) {
        self.cancel();
        b4n_common::tasks::wait_for_task(self.task.take(), "external bridge");
    }

    /// Returns vt100 parser used by this bridge instance.
    pub fn get_parser(&self) -> Arc<RwLock<vt100::Parser>> {
        Arc::clone(&self.parser)
    }

    /// Sends raw bytes to the process stdin.
    pub fn send(&self, data: Vec<u8>) {
        if self.is_running()
            && let Some(tx) = &self.input_tx
            && let Err(err) = tx.send(data)
        {
            tracing::warn!("Cannot send data to external process: {}", err);
        }
    }

    /// Resizes the PTY.
    pub fn set_terminal_size(&self, width: u16, height: u16) {
        if self.is_running()
            && let Some(tx) = &self.size_tx
        {
            let _ = tx.send((width, height));
        }
    }

    /// Returns `true` if the process is currently running.
    pub fn is_running(&self) -> bool {
        self.was_started && self.task.as_ref().is_some_and(|t| !t.is_finished()) && self.state.is_running()
    }

    /// Returns `true` if the process has finished.
    pub fn is_finished(&self) -> bool {
        (self.was_started && self.task.is_none()) || self.task.as_ref().is_some_and(JoinHandle::is_finished)
    }

    /// Returns `true` if the process has/had an error state.
    pub fn has_error(&self) -> bool {
        self.state.has_error()
    }

    /// Returns whether the terminal is in application cursor key mode.
    pub fn is_application_mode(&self) -> Option<bool> {
        match self.state.cursor_key_mode() {
            0 => None,
            1 => Some(false),
            _ => Some(true),
        }
    }

    /// Returns whether the terminal has mouse reporting enabled.
    pub fn is_mouse_enabled(&self) -> Option<bool> {
        match self.state.mouse_mode() {
            0 => None,
            1 => Some(false),
            _ => Some(true),
        }
    }
}

impl Drop for CmdBridge {
    fn drop(&mut self) {
        self.cancel();
    }
}

/// Waits for child process and returns `true` if it ended with an error.
async fn wait_for_child(command: &str, mut child: Box<dyn Child + Send + Sync>) -> bool {
    let exit_status = tokio::task::spawn_blocking(move || child.wait()).await;
    match exit_status {
        Ok(Ok(status)) if !status.success() => {
            tracing::warn!("'{}' exited with non-zero status", command);
            return true;
        },
        Ok(Err(err)) => {
            tracing::warn!("Error waiting for '{}': {}", command, err);
            return true;
        },
        Err(err) => {
            tracing::warn!("Task join error for '{}': {}", command, err);
            return true;
        },
        _ => {},
    }

    false
}

async fn input_bridge(
    mut writer: Box<dyn Write + Send>,
    mut input_rx: UnboundedReceiver<Vec<u8>>,
    cancellation_token: CancellationToken,
) {
    while !cancellation_token.is_cancelled() {
        tokio::select! {
            () = cancellation_token.cancelled() => break,
            Some(data) = input_rx.recv() => {
                if let Err(err) = writer.write_all(&data) {
                    tracing::warn!("Cannot write to PTY master: {}", err);
                    cancellation_token.cancel();
                    return;
                }
                if let Err(err) = writer.flush() {
                    tracing::warn!("Cannot flush PTY master: {}", err);
                    cancellation_token.cancel();
                    return;
                }
            }
        }
    }
}

async fn output_bridge(
    reader: Box<dyn Read + Send>,
    parser: Arc<RwLock<vt100::Parser>>,
    cancellation_token: CancellationToken,
    response_tx: UnboundedSender<Vec<u8>>,
    mut state: TerminalState,
    size: TerminalSize,
) -> bool {
    let (tx, mut rx) = mpsc::unbounded_channel::<Vec<u8>>();

    tokio::task::spawn_blocking(move || {
        let mut reader = reader;
        let mut buf = [0u8; 8192];

        loop {
            match reader.read(&mut buf) {
                Ok(0) => {
                    break;
                },
                Ok(n) => {
                    if tx.send(buf[..n].to_vec()).is_err() {
                        break;
                    }
                },
                Err(err) => {
                    tracing::debug!("PTY read ended: {}", err);
                    break;
                },
            }
        }
    });

    let mut total_bytes = 0usize;
    while let Some(data) = rx.recv().await {
        state.set_running(true);
        total_bytes += data.len();

        update_terminal_state(&data, &mut state);
        let response = handle_terminal_queries(&data, &parser, &size);
        let _ = response_tx.send(response);

        if let Ok(mut p) = parser.write() {
            p.process(&data);
        }
    }

    cancellation_token.cancel();
    total_bytes == 0
}

async fn resize_bridge(
    master: Arc<Mutex<Box<dyn portable_pty::MasterPty + Send>>>,
    mut size_rx: UnboundedReceiver<(u16, u16)>,
    cancellation_token: CancellationToken,
) {
    while !cancellation_token.is_cancelled() {
        tokio::select! {
            () = cancellation_token.cancelled() => break,
            Some((width, height)) = size_rx.recv() => {
                let size = portable_pty::PtySize {
                    rows: height,
                    cols: width,
                    ..Default::default()
                };

                if let Ok(master) = master.lock()
                    && let Err(err) = master.resize(size)
                {
                    tracing::warn!("Cannot resize PTY: {}", err);
                }
            }
        }
    }
}
