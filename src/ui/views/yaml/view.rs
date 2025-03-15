use clipboard::{ClipboardContext, ClipboardProvider};
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{Frame, layout::Rect};
use std::rc::Rc;
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    app::{SharedAppData, commands::CommandResult},
    kubernetes::Namespace,
    ui::{
        ResponseEvent, Responsive, TuiEvent,
        views::View,
        widgets::{Action, ActionsListBuilder, CommandPalette, FooterMessage},
    },
};

use super::YamlViewer;

/// YAML view.
pub struct YamlView {
    pub yaml: YamlViewer,
    app_data: SharedAppData,
    lines: Vec<String>,
    command_id: Option<String>,
    command_palette: CommandPalette,
    footer_tx: UnboundedSender<FooterMessage>,
}

impl YamlView {
    /// Creates new [`YamlView`] instance.
    pub fn new(
        app_data: SharedAppData,
        command_id: Option<String>,
        name: String,
        namespace: Namespace,
        kind_plural: String,
        footer_tx: UnboundedSender<FooterMessage>,
    ) -> Self {
        let viewer = YamlViewer::new(Rc::clone(&app_data), name, namespace, kind_plural);

        Self {
            yaml: viewer,
            app_data,
            lines: Vec::new(),
            command_id,
            command_palette: CommandPalette::default(),
            footer_tx,
        }
    }

    fn copy_yaml_to_clipboard(&mut self) {
        let result: Result<ClipboardContext, _> = ClipboardProvider::new();
        if let Ok(mut ctx) = result {
            if ctx.set_contents(self.lines.join("")).is_ok() {
                self.footer_tx
                    .send(FooterMessage::info(
                        " YAML content copied to the clipboard…".to_owned(),
                        1_500,
                    ))
                    .unwrap();
            }
        }
    }

    fn process_command_palette_events(&mut self, key: crossterm::event::KeyEvent) -> bool {
        if key.code == KeyCode::Char(':') || key.code == KeyCode::Char('>') {
            let actions = ActionsListBuilder::default()
                .with_close()
                .with_quit()
                .with_action(
                    Action::new("copy")
                        .with_description("copies YAML to the clipboard")
                        .with_response(ResponseEvent::Action("copy".to_owned())),
                )
                .build();
            self.command_palette = CommandPalette::new(Rc::clone(&self.app_data), actions, 60);
            self.command_palette.show();
            true
        } else {
            false
        }
    }
}

impl View for YamlView {
    fn command_id(&self) -> Option<&str> {
        self.command_id.as_deref()
    }

    fn process_command_result(&mut self, result: CommandResult) {
        if let CommandResult::ResourceYaml(Ok(result)) = result {
            self.yaml.set_header(result.name, result.namespace, result.kind_plural);
            self.yaml.set_content(
                result.styled,
                result.yaml.iter().map(|l| l.chars().count()).max().unwrap_or(0),
            );
            self.lines = result.yaml;
        }
    }

    fn process_event(&mut self, event: TuiEvent) -> ResponseEvent {
        let TuiEvent::Key(key) = event;

        if key.code == KeyCode::Char('c') && key.modifiers == KeyModifiers::CONTROL {
            return ResponseEvent::ExitApplication;
        }

        if self.command_palette.is_visible {
            let response = self.command_palette.process_key(key);
            if response.is_action("copy") {
                self.copy_yaml_to_clipboard();
                return ResponseEvent::Handled;
            }

            return response;
        }

        if self.process_command_palette_events(key) {
            return ResponseEvent::Handled;
        }

        if key.code == KeyCode::Char('c') {
            self.copy_yaml_to_clipboard();
            return ResponseEvent::Handled;
        }

        if key.code == KeyCode::Esc {
            return ResponseEvent::Cancelled;
        }

        self.yaml.process_key(key)
    }

    fn draw(&mut self, frame: &mut Frame<'_>, area: Rect) {
        self.yaml.draw(frame, area);
        self.command_palette.draw(frame, frame.area());
    }
}
