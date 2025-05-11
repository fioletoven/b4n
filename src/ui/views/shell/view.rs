use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{Frame, layout::Rect};
use std::sync::{Arc, RwLock};
use tui_term::{vt100, widget::PseudoTerminal};

use crate::{
    app::{SharedAppData, commands::CommandResult},
    kubernetes::{Namespace, PodRef, client::KubernetesClient},
    ui::{ResponseEvent, TuiEvent, views::View},
};

use super::bridge::{IOBridge, IOBridgeError};

/// Pod's shell view.
pub struct ShellView {
    app_data: SharedAppData,
    bridge: IOBridge,
    parser: Arc<RwLock<vt100::Parser>>,
}

impl ShellView {
    /// Creates new [`ShellView`] instance.
    pub fn new(
        app_data: SharedAppData,
        client: &KubernetesClient,
        pod_name: String,
        pod_namespace: Namespace,
        pod_container: Option<String>,
    ) -> Result<Self, IOBridgeError> {
        let pod = PodRef {
            name: pod_name.clone(),
            namespace: pod_namespace.clone(),
            container: pod_container.clone(),
        };
        let parser = Arc::new(RwLock::new(vt100::Parser::new(24, 80, 0)));

        let mut bridge = IOBridge::new(parser.clone());
        bridge.start(client, pod)?;

        Ok(Self {
            app_data,
            bridge,
            parser,
        })
    }
}

impl View for ShellView {
    fn process_command_result(&mut self, _result: CommandResult) {
        // pass
    }

    fn process_disconnection(&mut self) {
        // pass
    }

    fn process_event(&mut self, event: TuiEvent) -> ResponseEvent {
        let TuiEvent::Key(key) = event;

        if key.code == KeyCode::Char('c') && key.modifiers == KeyModifiers::CONTROL {
            return ResponseEvent::ExitApplication;
        }

        if key.code == KeyCode::Esc || !self.bridge.is_running() {
            return ResponseEvent::Cancelled;
        }

        match key.code {
            KeyCode::Char(input) => self.bridge.send(input.to_string().into_bytes()),
            KeyCode::Backspace => self.bridge.send(vec![8]),
            KeyCode::Enter => self.bridge.send(vec![b'\n']),
            KeyCode::Left => self.bridge.send(vec![27, 91, 68]),
            KeyCode::Right => self.bridge.send(vec![27, 91, 67]),
            KeyCode::Up => self.bridge.send(vec![27, 91, 65]),
            KeyCode::Down => self.bridge.send(vec![27, 91, 66]),
            KeyCode::Home => self.bridge.send(vec![27, 91, 72]),
            KeyCode::End => self.bridge.send(vec![27, 91, 70]),
            KeyCode::PageUp => self.bridge.send(vec![27, 91, 53, 126]),
            KeyCode::PageDown => self.bridge.send(vec![27, 91, 54, 126]),
            KeyCode::Tab => self.bridge.send(vec![9]),
            KeyCode::BackTab => self.bridge.send(vec![27, 91, 90]),
            KeyCode::Delete => self.bridge.send(vec![27, 91, 51, 126]),
            KeyCode::Insert => self.bridge.send(vec![27, 91, 50, 126]),
            _ => (),
        }

        ResponseEvent::Handled
    }

    fn draw(&mut self, frame: &mut Frame<'_>, area: Rect) {
        if !self.bridge.is_running() {
            return;
        }

        if let Ok(parser) = self.parser.read() {
            let screen = parser.screen();
            let pseudo_term = PseudoTerminal::new(screen);
            frame.render_widget(pseudo_term, area);
        }
    }
}
