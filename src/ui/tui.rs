use anyhow::Result;
use futures::{FutureExt, StreamExt};
use ratatui::{
    crossterm::{
        self, cursor,
        event::{Event, KeyEvent, KeyEventKind},
        terminal::{EnterAlternateScreen, LeaveAlternateScreen},
    },
    prelude::CrosstermBackend,
    Terminal,
};
use std::io::stdout;
use tokio::{
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;

use crate::app::utils::wait_for_task;

use super::utils::init_panic_hook;

/// Terminal UI Event
#[derive(Clone, Debug)]
pub enum TuiEvent {
    Key(KeyEvent),
}

/// Terminal UI Response Event.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum ResponseEvent {
    #[default]
    NotHandled,
    Handled,
    Cancelled,

    ExitApplication,

    Change(String, String),
    ChangeKind(String),
    ChangeNamespace(String),
    ViewNamespaces(String),

    ListKubeContexts,

    AskDeleteResources,
    DeleteResources,
}

/// Terminal UI.
pub struct Tui {
    pub terminal: Terminal<CrosstermBackend<std::io::Stdout>>,
    pub events_ct: CancellationToken,
    pub events_task: Option<JoinHandle<()>>,
    pub event_rx: UnboundedReceiver<TuiEvent>,
    pub event_tx: UnboundedSender<TuiEvent>,
}

impl Tui {
    /// Creates new [`Tui`] instance.
    pub fn new() -> Result<Self> {
        init_panic_hook();

        let (event_tx, event_rx) = mpsc::unbounded_channel();
        Ok(Self {
            terminal: Terminal::new(CrosstermBackend::new(stdout()))?,
            events_ct: CancellationToken::new(),
            events_task: None,
            event_rx,
            event_tx,
        })
    }

    /// Enters the alternate screen mode and starts terminal events loop.
    pub fn enter_terminal(&mut self) -> Result<()> {
        crossterm::terminal::enable_raw_mode()?;
        crossterm::execute!(stdout(), EnterAlternateScreen, cursor::Hide)?;
        self.start_events_loop();

        Ok(())
    }

    /// Exits the alternate screen mode and stops terminal events loop.
    pub fn exit_terminal(&mut self) -> Result<()> {
        self.stop_events_loop()?;
        if crossterm::terminal::is_raw_mode_enabled()? {
            self.terminal.flush()?;
            crossterm::execute!(stdout(), LeaveAlternateScreen, cursor::Show)?;
            crossterm::terminal::disable_raw_mode()?;
        }

        Ok(())
    }

    /// Cancels terminal events loop.
    pub fn cancel(&mut self) {
        self.events_ct.cancel();
    }

    /// Starts terminal events loop.
    pub fn start_events_loop(&mut self) {
        self.events_ct.cancel();
        self.events_ct = CancellationToken::new();
        let _cancellation_token = self.events_ct.clone();
        let _event_tx = self.event_tx.clone();
        let task = tokio::spawn(async move {
            let mut reader = crossterm::event::EventStream::new();
            loop {
                let crossterm_event = reader.next().fuse();
                tokio::select! {
                    _ = _cancellation_token.cancelled() => {
                        break;
                    },
                    maybe_event = crossterm_event => {
                        match maybe_event {
                            Some(Ok(event)) => process_crossterm_event(event, &_event_tx),
                            Some(Err(_)) => {},
                            None => {},
                        }
                    },
                }
            }
        });

        self.events_task = Some(task);
    }

    /// Stops terminal events loop.
    pub fn stop_events_loop(&mut self) -> Result<()> {
        self.events_ct.cancel();
        wait_for_task(self.events_task.take(), "events");

        Ok(())
    }
}

impl Drop for Tui {
    fn drop(&mut self) {
        self.exit_terminal().unwrap();
    }
}

fn process_crossterm_event(event: Event, sender: &UnboundedSender<TuiEvent>) {
    if let Event::Key(key) = event {
        if key.kind == KeyEventKind::Press {
            sender.send(TuiEvent::Key(key)).unwrap();
        }
    }
}
