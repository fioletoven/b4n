use b4n_common::NotificationSink;
use b4n_kube::{ContainerType, ResourceTag};
use b4n_tasks::commands::CommandResult;
use b4n_tui::{ResponseEvent, TuiEvent};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{Duration, Instant};
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
    fn draw(&mut self, frame: &mut Frame<'_>, area: Rect, has_focus: bool);
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

/// Calculates layout for view with header.
pub fn get_layout_with_header(area: Rect) -> Rc<[Rect]> {
    Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![Constraint::Length(1), Constraint::Fill(1)])
        .split(area)
}

/// Gets names for all containers in the list of resource tags.
pub fn get_all_containers_from_resource_tags(tags: &[ResourceTag]) -> Vec<String> {
    tags.iter()
        .filter_map(|t| match t {
            ResourceTag::Container(name, _, _) => Some(name.to_ascii_lowercase()),
            _ => None,
        })
        .collect()
}

/// Gets names for all non-ephemeral containers in the list of resource tags.\
/// **Note** that it sorts results by container type and then by name.
pub fn get_target_containers_from_resource_tags(tags: &[ResourceTag]) -> Vec<String> {
    let mut containers: Vec<(ContainerType, String)> = tags
        .iter()
        .filter_map(|t| match t {
            ResourceTag::Container(name, kind, _) if *kind != ContainerType::Ephemeral => {
                Some((*kind, name.to_ascii_lowercase()))
            },
            _ => None,
        })
        .collect();

    containers.sort_by(|(kind_a, name_a), (kind_b, name_b)| {
        let kind_order = |k: &ContainerType| match k {
            ContainerType::Regular => 0,
            ContainerType::Init => 1,
            ContainerType::Ephemeral => 2,
        };

        kind_order(kind_a).cmp(&kind_order(kind_b)).then(name_a.cmp(name_b))
    });

    containers.into_iter().map(|(_, name)| name).collect()
}

/// Tries to guess selected path for the file picker basing on the `is_download` flag.
pub fn get_path_for_file_picker(path: &str, is_download: bool) -> PathBuf {
    fn default_path() -> PathBuf {
        std::env::current_dir().unwrap_or(PathBuf::from("."))
    }

    if path.trim().is_empty() {
        default_path()
    } else {
        let path = PathBuf::from(path);
        if is_download {
            path
        } else if let Some(path) = path.parent()
            && !path.as_os_str().is_empty()
        {
            PathBuf::from(path)
        } else {
            default_path()
        }
    }
}

/// Allowed time between two key presses that will still activate the escape sequence mode.
pub const ESCAPE_SEQUENCE_TIMEOUT: Duration = Duration::from_millis(250);

/// Tracks whether the user triggered an escape sequence by pressing the same key twice.
/// Once active, the next key press is treated as an alternate command.
pub struct EscapeSequenceTracker {
    timeout: Duration,
    recorded_event: Option<TuiEvent>,
    last_press_time: Option<Instant>,
    is_active: bool,
}

impl EscapeSequenceTracker {
    /// Creates a new [`EscapeSequenceTracker`] with the given timeout between key presses.
    pub fn new(timeout: Duration) -> Self {
        Self {
            timeout,
            recorded_event: None,
            last_press_time: None,
            is_active: false,
        }
    }

    /// Records the trigger key event for the escape sequence. If the same key is pressed again
    /// within the allowed timeout, the escape sequence becomes active.
    pub fn record_event(&mut self, event: &TuiEvent) -> Option<TuiEvent> {
        if self.last_press_time.is_none_or(|t| t.elapsed() > self.timeout) {
            let prev_event = self.recorded_event.take();
            self.recorded_event = Some(event.clone());
            self.last_press_time = Some(Instant::now());
            self.is_active = false;
            prev_event
        } else {
            self.recorded_event = None;
            self.last_press_time = None;
            self.is_active = true;
            None
        }
    }

    /// Returns the recorded event if the escape sequence window has expired or if `force` is true.
    pub fn get_recorded_event(&mut self, force: bool) -> Option<TuiEvent> {
        if force || (self.recorded_event.is_some() && self.last_press_time.is_none_or(|t| t.elapsed() > self.timeout)) {
            self.recorded_event.take()
        } else {
            None
        }
    }

    /// Returns whether the escape sequence is currently active.
    pub fn is_active(&self) -> bool {
        self.is_active
    }

    /// Returns whether the escape sequence is currently active and resets the flag.
    pub fn consume_active(&mut self) -> bool {
        let was_active = self.is_active;
        self.is_active = false;
        was_active
    }
}
