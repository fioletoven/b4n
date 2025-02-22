use ratatui::Frame;
use ratatui::layout::Rect;

use crate::app::commands::CommandResult;

use super::{ResponseEvent, TuiEvent};

pub use self::resources::ResourcesView;
pub use self::yaml::YamlView;

mod resources;
mod yaml;

/// TUI view with pages and widgets.
pub trait View {
    /// Returns ID of the command associated with this [`View`].
    fn command_id(&self) -> Option<&str>;

    /// Returns `true` if provided command ID match the one associated with this [`View`].
    fn command_id_match(&self, command_id: &str) -> bool {
        self.command_id().is_some_and(|id| id == command_id)
    }

    /// Process result from the command.
    fn process_command_result(&mut self, result: CommandResult);

    /// Process single TUI event.
    fn process_event(&mut self, event: TuiEvent) -> ResponseEvent;

    /// Draw [`View`] on the provided frame and area.
    fn draw(&mut self, frame: &mut Frame<'_>, area: Rect);
}
