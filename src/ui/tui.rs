use anyhow::Result;
use crossterm::{
    cursor::SetCursorStyle,
    event::{EnableMouseCapture, KeyModifiers, MouseButton},
};
use futures::{FutureExt, StreamExt};
use ratatui::{
    Terminal,
    crossterm::{
        self, cursor,
        event::{Event, KeyEventKind},
        terminal::{EnterAlternateScreen, LeaveAlternateScreen},
    },
    layout::{Position, Rect},
    prelude::CrosstermBackend,
};
use std::{
    io::stdout,
    time::{Duration, Instant},
};
use tokio::{
    runtime::Handle,
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;

use crate::{core::utils::wait_for_task, kubernetes::ResourceRef, ui::KeyCombination};

use super::utils::init_panic_hook;

static DOUBLE_CLICK_DURATION: Duration = Duration::from_millis(300);

/// TUI mouse event.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct MouseEvent {
    pub kind: MouseEventKind,
    pub column: u16,
    pub row: u16,
    pub modifiers: KeyModifiers,
}

impl From<crossterm::event::MouseEvent> for MouseEvent {
    fn from(value: crossterm::event::MouseEvent) -> Self {
        Self {
            kind: match value.kind {
                crossterm::event::MouseEventKind::Down(button) => match button {
                    MouseButton::Left => MouseEventKind::LeftClick,
                    MouseButton::Right => MouseEventKind::RightClick,
                    MouseButton::Middle => MouseEventKind::MiddleClick,
                },
                crossterm::event::MouseEventKind::ScrollDown => MouseEventKind::ScrollDown,
                crossterm::event::MouseEventKind::ScrollUp => MouseEventKind::ScrollUp,
                crossterm::event::MouseEventKind::ScrollLeft => MouseEventKind::ScrollLeft,
                crossterm::event::MouseEventKind::ScrollRight => MouseEventKind::ScrollRight,
                _ => MouseEventKind::None,
            },
            column: value.column,
            row: value.row,
            modifiers: value.modifiers,
        }
    }
}

/// TUI mouse event kind.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum MouseEventKind {
    None,
    LeftClick,
    LeftDoubleClick,
    RightClick,
    RightDoubleClick,
    MiddleClick,
    MiddleDoubleClick,
    ScrollDown,
    ScrollUp,
    ScrollLeft,
    ScrollRight,
}

/// TUI event.
#[derive(Debug, Clone)]
pub enum TuiEvent {
    Key(KeyCombination),
    Mouse(MouseEvent),
}

impl TuiEvent {
    /// Returns the line number if this is a mouse event inside a specified area.
    pub fn get_clicked_line_no(&self, kind: MouseEventKind, modifiers: KeyModifiers, area: Rect) -> Option<u16> {
        if let TuiEvent::Mouse(mouse) = self
            && mouse.kind == kind
            && mouse.modifiers == modifiers
            && area.contains(Position::new(mouse.column, mouse.row))
        {
            Some(mouse.row.saturating_sub(area.y))
        } else {
            None
        }
    }

    /// Returns `true` if this event is a mouse event of a specified kind.
    pub fn is(&self, kind: MouseEventKind) -> bool {
        matches!(self, TuiEvent::Mouse(mouse) if mouse.kind == kind)
    }

    /// Returns `true` if this event is a mouse event of a specified kind in a specified area.
    pub fn is_in(&self, kind: MouseEventKind, area: Rect) -> bool {
        matches!(self, TuiEvent::Mouse(mouse) if mouse.kind == kind && area.contains(Position::new(mouse.column, mouse.row)))
    }

    /// Returns `true` if this event is a mouse event of a specified kind outsied of a specified area.
    pub fn is_out(&self, kind: MouseEventKind, area: Rect) -> bool {
        matches!(self, TuiEvent::Mouse(mouse) if mouse.kind == kind && !area.contains(Position::new(mouse.column, mouse.row)))
    }
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
    pub fn enter_terminal(&mut self, runtime: &Handle) -> Result<()> {
        crossterm::terminal::enable_raw_mode()?;
        crossterm::execute!(stdout(), EnterAlternateScreen, cursor::Hide)?;
        crossterm::execute!(stdout(), SetCursorStyle::SteadyBar)?;

        crossterm::execute!(stdout(), EnableMouseCapture)?;

        self.start_events_loop(runtime);

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
    pub fn start_events_loop(&mut self, runtime: &Handle) {
        self.events_ct.cancel();
        self.events_ct = CancellationToken::new();
        let _cancellation_token = self.events_ct.clone();
        let _event_tx = self.event_tx.clone();
        let task = runtime.spawn(async move {
            let mut click = DblClickState {
                button: MouseButton::Left,
                time: None,
            };
            let mut reader = crossterm::event::EventStream::new();
            loop {
                let crossterm_event = reader.next().fuse();
                tokio::select! {
                    () = _cancellation_token.cancelled() => {
                        break;
                    },
                    maybe_event = crossterm_event => {
                        if let Some(Ok(event)) = maybe_event {
                            click = process_crossterm_event(event, &_event_tx, click);
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

#[derive(Debug)]
struct DblClickState {
    button: MouseButton,
    time: Option<Instant>,
}

fn process_crossterm_event(event: Event, sender: &UnboundedSender<TuiEvent>, prev_click: DblClickState) -> DblClickState {
    match event {
        Event::Key(key) if key.kind == KeyEventKind::Press => {
            let _ = sender.send(TuiEvent::Key(key.into()));
            prev_click
        },

        Event::Mouse(mouse_event) => {
            let now = Instant::now();

            match mouse_event.kind {
                crossterm::event::MouseEventKind::Down(button) => {
                    let is_double_click = prev_click
                        .time
                        .filter(|&t| now.duration_since(t) <= DOUBLE_CLICK_DURATION)
                        .is_some()
                        && prev_click.button == button;

                    let mut event: MouseEvent = mouse_event.into();

                    if is_double_click {
                        event.kind = match button {
                            MouseButton::Left => MouseEventKind::LeftDoubleClick,
                            MouseButton::Right => MouseEventKind::RightDoubleClick,
                            MouseButton::Middle => MouseEventKind::MiddleDoubleClick,
                        };
                        let _ = sender.send(TuiEvent::Mouse(event));
                        DblClickState { time: None, button }
                    } else {
                        let _ = sender.send(TuiEvent::Mouse(event));
                        DblClickState { time: Some(now), button }
                    }
                },

                crossterm::event::MouseEventKind::ScrollUp
                | crossterm::event::MouseEventKind::ScrollDown
                | crossterm::event::MouseEventKind::ScrollLeft
                | crossterm::event::MouseEventKind::ScrollRight => {
                    let _ = sender.send(TuiEvent::Mouse(mouse_event.into()));
                    prev_click
                },

                _ => prev_click,
            }
        },

        _ => prev_click,
    }
}
