use anyhow::Result;
use crossterm::cursor::SetCursorStyle;
use futures::{FutureExt, StreamExt};
use ratatui::{
    Terminal,
    crossterm::{
        self, cursor,
        event::{Event, KeyEventKind},
        terminal::{EnterAlternateScreen, LeaveAlternateScreen},
    },
    prelude::CrosstermBackend,
};
use std::io::stdout;
use tokio::{
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;

use crate::{core::utils::wait_for_task, kubernetes::ResourceRef, ui::KeyCombination};

use super::utils::init_panic_hook;

/// Terminal UI Event.
#[derive(Clone)]
pub enum TuiEvent {
    Key(KeyCombination),
}

/// Terminal UI Response Event.
#[derive(Clone, Default, PartialEq)]
pub enum ResponseEvent {
    #[default]
    NotHandled,
    Handled,
    Cancelled,
    Accepted,
    Action(&'static str),

    ExitApplication,

    Change(String, String),
    ChangeKind(String),
    ChangeKindAndSelect(String, Option<String>),
    ChangeNamespace(String),
    ChangeContext(String),
    ChangeTheme(String),
    ViewContainers(String, String),
    ViewNamespaces,

    ListKubeContexts,
    ListThemes,
    ListResourcePorts(ResourceRef),

    AskDeleteResources,
    DeleteResources,

    ViewYaml(ResourceRef, bool),
    ViewLogs(ResourceRef),
    ViewPreviousLogs(ResourceRef),

    OpenShell(ResourceRef),
    ShowPortForwards,
    PortForward(ResourceRef, u16, u16, String),
}

impl ResponseEvent {
    /// Returns `true` if [`ResponseEvent`] is an action matching the provided name.
    pub fn is_action(&self, name: &str) -> bool {
        if let ResponseEvent::Action(action) = self {
            *action == name
        } else {
            false
        }
    }

    /// Conditionally converts [`ResponseEvent`] to a different [`ResponseEvent`] consuming it.\
    /// **Note** that the new instance is returned by the `f` closure executed only if it is an action matching the provided name.
    pub fn when_action_then<F>(self, name: &str, f: F) -> Self
    where
        F: FnOnce() -> Self,
    {
        if self.is_action(name) { f() } else { self }
    }
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
        crossterm::execute!(stdout(), SetCursorStyle::SteadyBar)?;
        self.start_events_loop();

        Ok(())
    }

    /// Exits the alternate screen mode and stops terminal events loop.
    pub fn exit_terminal(&mut self) -> Result<()> {
        self.stop_events_loop()?;
        if crossterm::terminal::is_raw_mode_enabled()? {
            self.terminal.flush()?;
            crossterm::execute!(stdout(), SetCursorStyle::DefaultUserShape)?;
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
                    () = _cancellation_token.cancelled() => {
                        break;
                    },
                    maybe_event = crossterm_event => {
                        if let Some(Ok(event)) = maybe_event { process_crossterm_event(event, &_event_tx) }
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
    if let Event::Key(key) = event
        && key.kind == KeyEventKind::Press
    {
        sender.send(TuiEvent::Key(key.into())).unwrap();
    }
}
