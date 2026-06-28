use b4n_common::NotificationSink;
use b4n_tasks::commands::CommandResult;
use b4n_tui::{ResponseEvent, TuiEvent};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use std::rc::Rc;
use std::time::Instant;
use tui_term::vt100::Screen;

use crate::core::{SharedAppData, SharedAppDataExt};
use crate::ui::presentation::ScreenSelection;

/// TUI view with pages and widgets.
pub trait View {
    /// Returns ID of the command associated with this [`View`].
    fn command_id(&self) -> Option<&str> {
        None
    }

    /// Returns `true` if provided command ID match the one associated with this [`View`].
    fn command_id_match(&self, command_id: &str) -> bool {
        self.command_id().is_some_and(|id| id == command_id)
    }

    /// Returns name of the namespace displayed on the view.\
    /// **Note** that this is used e.g. in side selector to highlight current namespace.
    fn displayed_namespace(&self) -> &str {
        ""
    }

    /// Returns `true` if namespaces selector can be displayed on the view.
    fn is_namespaces_selector_allowed(&self) -> bool {
        false
    }

    /// Returns `true` if resources selector can be displayed on the view.
    fn is_resources_selector_allowed(&self) -> bool {
        false
    }

    /// Handles event returned by the namespaces' selector.
    fn handle_namespaces_selector_event(&mut self, event: &ResponseEvent) {
        let _ = event;
    }

    /// Handles event returned by the resources' selector.
    fn handle_resources_selector_event(&mut self, event: &ResponseEvent) {
        let _ = event;
    }

    /// Handles a namespace change event.
    fn handle_namespace_change(&mut self) {}

    /// Handles a resource's kind change event.
    fn handle_kind_change(&mut self) {}

    /// Processes result from the command.
    fn process_command_result(&mut self, result: CommandResult) {
        let _ = result;
    }

    /// Processes app tick.
    fn process_tick(&mut self) -> ResponseEvent {
        ResponseEvent::Handled
    }

    /// Processes disconnection state.
    fn process_disconnection(&mut self);

    /// Processes single TUI event.
    fn process_event(&mut self, event: &TuiEvent) -> ResponseEvent;

    /// Draw [`View`] on the provided frame and area.
    fn draw(&mut self, frame: &mut Frame<'_>, area: Rect);
}

/// Extension methods for [`Screen`].
pub trait ScreenExt {
    /// Copies whole screen or a selection to the clipboard.
    fn copy_to_clipboard(&self, app_data: &mut SharedAppData, selection: &mut ScreenSelection, sink: &NotificationSink);
}

impl ScreenExt for Screen {
    fn copy_to_clipboard(&self, app_data: &mut SharedAppData, selection: &mut ScreenSelection, sink: &NotificationSink) {
        if let Some((start, end)) = selection.sorted() {
            let text = self.contents_between(start.y, start.x, end.y, end.x + 1);
            app_data.copy_to_clipboard(text, sink, || "Selected text copied to clipboard");
        } else {
            let text = self.contents();
            app_data.copy_to_clipboard(text, sink, || "Whole screen copied to clipboard");
        }

        selection.reset();
    }
}

/// Tracks `ESC` key press count.
pub struct EscPressTracker {
    esc_count: u8,
    esc_time: Instant,
}

impl Default for EscPressTracker {
    fn default() -> Self {
        Self {
            esc_count: 0,
            esc_time: Instant::now(),
        }
    }
}

impl EscPressTracker {
    /// Checks if `ESC` key was pressed quickly `x` times.
    pub fn is_pressed_times(&mut self, times: u8) -> bool {
        if self.esc_time.elapsed().as_millis() < (200 * u128::from(times)) {
            self.esc_count += 1;
        } else {
            self.esc_count = 1;
            self.esc_time = Instant::now();
        }

        if self.esc_count == times {
            self.esc_count = 0;
            true
        } else {
            false
        }
    }
}

/// Calculates layout for view with header.
pub fn get_layout_with_header(area: Rect) -> Rc<[Rect]> {
    Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![Constraint::Length(1), Constraint::Fill(1)])
        .split(area)
}
