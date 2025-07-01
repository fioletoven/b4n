use ratatui::Frame;
use ratatui::layout::Rect;

use crate::core::commands::CommandResult;

use super::{ResponseEvent, TuiEvent};

pub use self::forwards::{ForwardsView, PortForwardItem, PortForwardsList};
pub use self::list::ListViewer;
pub use self::list_header::ListHeader;
pub use self::logs::LogsView;
pub use self::resources::ResourcesView;
pub use self::shell::ShellView;
pub use self::utils::*;
pub use self::yaml::YamlView;

mod content;
mod content_header;
mod forwards;
mod list;
mod list_header;
mod logs;
mod resources;
mod shell;
mod utils;
mod yaml;

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

    /// Returns `true` if namespaces selector can be displayed on the view.
    fn is_namespaces_selector_allowed(&self) -> bool {
        false
    }

    /// Returns `true` if resources selector can be displayed on the view.
    fn is_resources_selector_allowed(&self) -> bool {
        false
    }

    /// Returns name of the namespace displayed on the view.\
    /// **Note** that this is used e.g. in side selector to highlight current namespace.
    fn displayed_namespace(&self) -> &str {
        ""
    }

    /// Processes namespace change.
    fn process_namespace_change(&mut self) {}

    /// Processes resource's kind change.
    fn process_kind_change(&mut self) {}

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
    fn process_event(&mut self, event: TuiEvent) -> ResponseEvent;

    /// Draw [`View`] on the provided frame and area.
    fn draw(&mut self, frame: &mut Frame<'_>, area: Rect);
}
