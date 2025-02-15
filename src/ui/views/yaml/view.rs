use std::rc::Rc;

use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::Frame;

use crate::{
    app::{commands::CommandResult, SharedAppData},
    kubernetes::Namespace,
    ui::{views::View, ResponseEvent, TuiEvent},
};

use super::YamlViewer;

/// YAML view.
pub struct YamlView {
    pub yaml: YamlViewer,
    command_id: Option<String>,
}

impl YamlView {
    /// Creates new [`YamlView`] instance.
    pub fn new(app_data: SharedAppData, command_id: Option<String>, name: String, namespace: Namespace) -> Self {
        let viewer = YamlViewer::new(Rc::clone(&app_data), name, namespace);
        Self {
            yaml: viewer,
            command_id,
        }
    }
}

impl View for YamlView {
    fn command_id(&self) -> Option<&str> {
        self.command_id.as_deref()
    }

    fn process_command_result(&mut self, result: CommandResult) {
        match result {
            CommandResult::ResourceYaml(Ok(result)) => {
                self.yaml.set_header(result.name, result.namespace);
                self.yaml.set_content(result.styled);
            }
            _ => (),
        }
    }

    fn process_event(&mut self, event: TuiEvent) -> ResponseEvent {
        let TuiEvent::Key(key) = event;

        if key.code == KeyCode::Char('c') && key.modifiers == KeyModifiers::CONTROL {
            return ResponseEvent::ExitApplication;
        }

        if key.code == KeyCode::Esc {
            return ResponseEvent::Cancelled;
        }

        self.yaml.process_key(key)
    }

    fn draw(&mut self, frame: &mut Frame<'_>) {
        self.yaml.draw(frame, frame.area());
    }
}
