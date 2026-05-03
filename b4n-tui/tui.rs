use anyhow::Result;
use b4n_config::keys::{KeyCombination, KeyCommand};
use crossterm::cursor::{self, SetCursorStyle};
use crossterm::event::{DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind, KeyModifiers, MouseButton};
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};
use ratatui_core::layout::{Position, Rect};
use ratatui_core::terminal::Terminal;
use ratatui_crossterm::CrosstermBackend;
use std::io::stdout;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio_util::sync::CancellationToken;

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
                crossterm::event::MouseEventKind::Up(button) => match button {
                    MouseButton::Left => MouseEventKind::LeftUp,
                    MouseButton::Right => MouseEventKind::RightUp,
                    MouseButton::Middle => MouseEventKind::MiddleUp,
                },
                crossterm::event::MouseEventKind::Drag(button) => match button {
                    MouseButton::Left => MouseEventKind::LeftDrag,
                    MouseButton::Right => MouseEventKind::RightDrag,
                    MouseButton::Middle => MouseEventKind::MiddleDrag,
                },
                crossterm::event::MouseEventKind::Moved => MouseEventKind::Moved,
                crossterm::event::MouseEventKind::ScrollDown => MouseEventKind::ScrollDown,
                crossterm::event::MouseEventKind::ScrollUp => MouseEventKind::ScrollUp,
                crossterm::event::MouseEventKind::ScrollLeft => MouseEventKind::ScrollLeft,
                crossterm::event::MouseEventKind::ScrollRight => MouseEventKind::ScrollRight,
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
    LeftClick,
    LeftDoubleClick,
    LeftTripleClick,
    LeftUp,
    LeftDrag,
    RightClick,
    RightDoubleClick,
    RightTripleClick,
    RightUp,
    RightDrag,
    MiddleClick,
    MiddleDoubleClick,
    MiddleTripleClick,
    MiddleUp,
    MiddleDrag,
    Moved,
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
    Command(KeyCommand),
}

impl TuiEvent {
    /// Returns the line number if the mouse event matches the specified kind, modifiers, and is within the given area.
    pub fn get_line_no(&self, kind: MouseEventKind, modifiers: KeyModifiers, area: Rect) -> Option<u16> {
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

    /// Returns `true` if this event is a key event matching specified combination.
    pub fn is_key(&self, combination: &KeyCombination) -> bool {
        matches!(self, TuiEvent::Key(key) if key == combination)
    }

    /// Returns `true` if this event is a mouse event of a specified kind.
    pub fn is_mouse(&self, kind: MouseEventKind) -> bool {
        matches!(self, TuiEvent::Mouse(mouse) if mouse.kind == kind)
    }

    /// Returns `true` if this event is a mouse event of a specified kind inside a specified area.
    pub fn is_in(&self, kind: MouseEventKind, area: Rect) -> bool {
        matches!(self, TuiEvent::Mouse(mouse) if mouse.kind == kind && area.contains(Position::new(mouse.column, mouse.row)))
    }

    /// Returns `true` if this event is a mouse event of a specified kind outside a specified area.
    pub fn is_out(&self, kind: MouseEventKind, area: Rect) -> bool {
        matches!(self, TuiEvent::Mouse(mouse) if mouse.kind == kind && !area.contains(Position::new(mouse.column, mouse.row)))
    }

    /// Returns mouse position if this event is a mouse event.
    pub fn position(&self) -> Option<Position> {
        match self {
            TuiEvent::Mouse(mouse) => Some(Position::new(mouse.column, mouse.row)),
            TuiEvent::Key(_) | TuiEvent::Command(_) => None,
        }
    }
}

impl From<KeyCombination> for TuiEvent {
    fn from(value: KeyCombination) -> Self {
        TuiEvent::Key(value)
    }
}

/// Terminal UI.
pub struct Tui {
    pub terminal: Terminal<CrosstermBackend<std::io::Stdout>>,
    pub events_ct: CancellationToken,
    pub events_task: Option<JoinHandle<()>>,
    pub event_rx: UnboundedReceiver<TuiEvent>,
    pub event_tx: UnboundedSender<TuiEvent>,
    is_mouse_enabled: bool,
}

impl Tui {
    /// Creates new [`Tui`] instance.
    pub fn new(is_mouse_enabled: bool) -> Result<Self> {
        init_panic_hook();

        let (event_tx, event_rx) = mpsc::unbounded_channel();
        Ok(Self {
            terminal: Terminal::new(CrosstermBackend::new(stdout()))?,
            events_ct: CancellationToken::new(),
            events_task: None,
            event_rx,
            event_tx,
            is_mouse_enabled,
        })
    }

    /// Enters the alternate screen mode and starts terminal events loop.
    pub fn enter_terminal(&mut self) -> Result<()> {
        crossterm::terminal::enable_raw_mode()?;
        crossterm::execute!(stdout(), EnterAlternateScreen, SetCursorStyle::SteadyBar, cursor::Hide)?;
        if self.is_mouse_enabled {
            crossterm::execute!(stdout(), EnableMouseCapture)?;
        }

        self.start_events_loop()?;

        Ok(())
    }

    /// Exits the alternate screen mode and stops terminal events loop.
    pub fn exit_terminal(&mut self) -> Result<()> {
        self.stop_events_loop()?;
        if crossterm::terminal::is_raw_mode_enabled()? {
            self.terminal.flush()?;
            crossterm::execute!(stdout(), LeaveAlternateScreen, SetCursorStyle::DefaultUserShape, cursor::Show)?;
            if self.is_mouse_enabled {
                crossterm::execute!(stdout(), DisableMouseCapture)?;
            }

            crossterm::terminal::disable_raw_mode()?;
        }

        Ok(())
    }

    /// Enables or disables mouse capture in terminal.
    pub fn toggle_mouse_support(&mut self) -> Result<()> {
        if crossterm::terminal::is_raw_mode_enabled()? {
            self.is_mouse_enabled = !self.is_mouse_enabled;
            if self.is_mouse_enabled {
                crossterm::execute!(stdout(), EnableMouseCapture)?;
            } else {
                crossterm::execute!(stdout(), DisableMouseCapture)?;
            }
        }

        Ok(())
    }

    /// Returns `true` if mouse support is enabled in the terminal.
    pub fn is_mouse_enabled(&self) -> bool {
        self.is_mouse_enabled
    }

    /// Cancels terminal events loop.
    pub fn cancel(&mut self) {
        self.events_ct.cancel();
    }

    /// Starts terminal events loop on a dedicated thread.
    pub fn start_events_loop(&mut self) -> Result<()> {
        self.events_ct.cancel();
        self.events_ct = CancellationToken::new();
        let _cancellation_token = self.events_ct.clone();
        let _event_tx = self.event_tx.clone();

        let task = std::thread::Builder::new().name("tui-events".to_string()).spawn(move || {
            let mut click = DblClickState::default();

            while !_cancellation_token.is_cancelled() {
                match crossterm::event::poll(Duration::from_millis(100)) {
                    Ok(true) => {
                        if let Ok(event) = crossterm::event::read() {
                            click = process_crossterm_event(event, &_event_tx, click);
                        }
                    },
                    _ => continue,
                }
            }
        })?;

        self.events_task = Some(task);

        Ok(())
    }

    /// Stops terminal events loop.
    pub fn stop_events_loop(&mut self) -> Result<()> {
        self.events_ct.cancel();
        if let Some(handle) = self.events_task.take() {
            let _ = handle.join();
        }

        Ok(())
    }
}

impl Drop for Tui {
    fn drop(&mut self) {
        let _ = self.exit_terminal();
    }
}

#[derive(Debug)]
struct DblClickState {
    button: MouseButton,
    time: Option<Instant>,
    count: u8,
}

impl Default for DblClickState {
    fn default() -> Self {
        Self {
            button: MouseButton::Left,
            time: None,
            count: 0,
        }
    }
}

impl DblClickState {
    fn new(time: Instant, button: MouseButton, count: u8) -> Self {
        Self {
            button,
            time: Some(time),
            count,
        }
    }
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
                    let click_no = if prev_click.button == button
                        && prev_click
                            .time
                            .is_some_and(|t| now.duration_since(t) <= DOUBLE_CLICK_DURATION)
                    {
                        prev_click.count.wrapping_add(1)
                    } else {
                        1
                    };

                    let mut event: MouseEvent = mouse_event.into();
                    match click_no {
                        2 => {
                            event.kind = match button {
                                MouseButton::Left => MouseEventKind::LeftDoubleClick,
                                MouseButton::Right => MouseEventKind::RightDoubleClick,
                                MouseButton::Middle => MouseEventKind::MiddleDoubleClick,
                            };
                        },
                        3 => {
                            event.kind = match button {
                                MouseButton::Left => MouseEventKind::LeftTripleClick,
                                MouseButton::Right => MouseEventKind::RightTripleClick,
                                MouseButton::Middle => MouseEventKind::MiddleTripleClick,
                            };
                        },
                        _ => (),
                    }

                    let _ = sender.send(TuiEvent::Mouse(event));
                    DblClickState::new(now, button, click_no)
                },

                crossterm::event::MouseEventKind::Drag(_)
                | crossterm::event::MouseEventKind::Moved
                | crossterm::event::MouseEventKind::ScrollUp
                | crossterm::event::MouseEventKind::ScrollDown
                | crossterm::event::MouseEventKind::ScrollLeft
                | crossterm::event::MouseEventKind::ScrollRight => {
                    let _ = sender.send(TuiEvent::Mouse(mouse_event.into()));
                    DblClickState::default()
                },

                crossterm::event::MouseEventKind::Up(_) => {
                    let _ = sender.send(TuiEvent::Mouse(mouse_event.into()));
                    prev_click
                },
            }
        },

        _ => prev_click,
    }
}
