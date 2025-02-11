use ratatui::Frame;

use super::{ResponseEvent, TuiEvent};

pub use self::resources::*;
pub use self::yaml::*;

mod resources;
mod yaml;

pub trait View {
    fn process_event(&mut self, event: TuiEvent) -> ResponseEvent;
    fn draw(&mut self, frame: &mut Frame<'_>);
}
