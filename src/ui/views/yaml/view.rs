use clipboard::{ClipboardContext, ClipboardProvider};
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{Frame, layout::Rect};
use std::rc::Rc;
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    core::{SharedAppData, SharedBgWorker, commands::CommandResult},
    kubernetes::{Kind, Namespace, resources::SECRETS},
    ui::{
        ResponseEvent, Responsive, TuiEvent,
        views::{
            View,
            content::{Content, ContentViewer, StyledLine},
        },
        widgets::{ActionItem, ActionsListBuilder, CommandPalette, FooterMessage},
    },
};

/// YAML view.
pub struct YamlView {
    yaml: ContentViewer<YamlContent>,
    app_data: SharedAppData,
    worker: SharedBgWorker,
    lines: Vec<String>,
    is_decoded: bool,
    command_id: Option<String>,
    command_palette: CommandPalette,
    footer_tx: UnboundedSender<FooterMessage>,
}

impl YamlView {
    /// Creates new [`YamlView`] instance.
    pub fn new(
        app_data: SharedAppData,
        worker: SharedBgWorker,
        command_id: Option<String>,
        name: String,
        namespace: Namespace,
        kind: Kind,
        footer_tx: UnboundedSender<FooterMessage>,
    ) -> Self {
        let yaml = ContentViewer::new(Rc::clone(&app_data)).with_header("YAML", '', namespace, kind, name, None);

        Self {
            yaml,
            app_data,
            worker,
            lines: Vec::new(),
            is_decoded: false,
            command_id,
            command_palette: CommandPalette::default(),
            footer_tx,
        }
    }

    fn copy_yaml_to_clipboard(&mut self) {
        let result: Result<ClipboardContext, _> = ClipboardProvider::new();
        if let Ok(mut ctx) = result
            && ctx.set_contents(self.lines.join("")).is_ok()
        {
            self.footer_tx
                .send(FooterMessage::info(" YAML content copied to the clipboard…", 1_500))
                .unwrap();
        }
    }

    fn process_command_palette_events(&mut self, key: crossterm::event::KeyEvent) -> bool {
        if key.code == KeyCode::Char(':') || key.code == KeyCode::Char('>') {
            let mut builder = ActionsListBuilder::default().with_close().with_quit().with_action(
                ActionItem::new("copy")
                    .with_description("copies YAML to the clipboard")
                    .with_response(ResponseEvent::Action("copy")),
            );
            if self.yaml.header.kind.as_str() == SECRETS && self.app_data.borrow().is_connected {
                let action = if self.is_decoded { "encode" } else { "decode" };
                builder = builder.with_action(
                    ActionItem::new(action)
                        .with_description(&format!("{action}s the resource's data"))
                        .with_response(ResponseEvent::Action("decode")),
                );
            }

            self.command_palette = CommandPalette::new(Rc::clone(&self.app_data), builder.build(), 60);
            self.command_palette.show();
            true
        } else {
            false
        }
    }

    fn toggle_yaml_decode(&mut self) {
        self.command_id = self.worker.borrow_mut().get_yaml(
            self.yaml.header.name.clone(),
            self.yaml.header.namespace.clone(),
            &self.yaml.header.kind,
            self.app_data.borrow().get_syntax_data(),
            !self.is_decoded,
        );
    }
}

impl View for YamlView {
    fn command_id(&self) -> Option<&str> {
        self.command_id.as_deref()
    }

    fn process_command_result(&mut self, result: CommandResult) {
        if let CommandResult::ResourceYaml(Ok(result)) = result {
            let icon = if result.is_decoded { '' } else { '' };
            self.is_decoded = result.is_decoded;
            self.yaml.header.set_icon(icon);
            self.yaml.header.set_data(result.namespace, result.kind, result.name, None);
            self.yaml.set_content(
                YamlContent { lines: result.styled },
                result.yaml.iter().map(|l| l.chars().count()).max().unwrap_or(0),
            );
            self.lines = result.yaml;
        }
    }

    fn process_disconnection(&mut self) {
        self.command_palette.hide();
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
            } else if response.is_action("decode") {
                self.toggle_yaml_decode();
                return ResponseEvent::Handled;
            }

            return response;
        }

        if self.process_command_palette_events(key) {
            return ResponseEvent::Handled;
        }

        if key.code == KeyCode::Char('x') && self.yaml.header.kind.as_str() == SECRETS && self.app_data.borrow().is_connected {
            self.toggle_yaml_decode();
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

/// Styled YAML content.
struct YamlContent {
    lines: Vec<StyledLine>,
}

impl Content for YamlContent {
    fn page(&mut self, start: usize, count: usize) -> &[StyledLine] {
        if start >= self.lines.len() {
            &[]
        } else if start + count >= self.lines.len() {
            &self.lines[start..]
        } else {
            &self.lines[start..start + count]
        }
    }

    fn len(&self) -> usize {
        self.lines.len()
    }
}
