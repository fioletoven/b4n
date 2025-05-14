use crossterm::event::{KeyCode, KeyModifiers};
use kube::Client;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
};
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc::UnboundedSender;
use tui_term::{vt100, widget::PseudoTerminal};

use crate::{
    app::SharedAppData,
    kubernetes::{Namespace, PodRef, client::KubernetesClient, resources::PODS},
    ui::{
        ResponseEvent, TuiEvent,
        views::{View, header::HeaderPane},
        widgets::FooterMessage,
    },
};

use super::bridge::{ShellBridge, ShellBridgeError};

const DEFAULT_SHELL: &str = "bash";
const FALLBACK_SHELL: &str = "sh";

/// Pod's shell view.
pub struct ShellView {
    header: HeaderPane,
    bridge: ShellBridge,
    parser: Arc<RwLock<vt100::Parser>>,
    size: (u16, u16), // vt size (width, height)
    client: Client,
    pod: PodRef,
    footer_tx: UnboundedSender<FooterMessage>,
}

impl ShellView {
    /// Creates new [`ShellView`] instance.
    pub fn new(
        app_data: SharedAppData,
        client: &KubernetesClient,
        pod_name: String,
        pod_namespace: Namespace,
        pod_container: Option<String>,
        footer_tx: UnboundedSender<FooterMessage>,
    ) -> Result<Self, ShellBridgeError> {
        let pod = PodRef {
            name: pod_name.clone(),
            namespace: pod_namespace.clone(),
            container: pod_container.clone(),
        };
        let mut header = HeaderPane::new(app_data, false);
        header.set_title(" shell");
        header.set_data(pod_namespace, PODS.into(), pod_name, pod_container);

        let parser = Arc::new(RwLock::new(vt100::Parser::new(24, 80, 0)));
        let mut bridge = ShellBridge::new(parser.clone());
        bridge.start(client.get_client(), pod.clone(), DEFAULT_SHELL)?;

        Ok(Self {
            header,
            bridge,
            parser,
            size: (0, 0),
            client: client.get_client(),
            pod,
            footer_tx,
        })
    }
}

impl View for ShellView {
    fn process_tick(&mut self) -> ResponseEvent {
        if self.bridge.is_finished() {
            // we try to fallback to 'sh' if ShellBridge has an error and was initially started as 'bash'
            if self.bridge.has_error() && self.bridge.shell().is_some_and(|s| s == DEFAULT_SHELL) {
                let _ = self.bridge.start(self.client.clone(), self.pod.clone(), FALLBACK_SHELL);
                ResponseEvent::Handled
            } else {
                if self.bridge.has_error() {
                    self.footer_tx
                        .send(FooterMessage::error(
                            "Unable to attach to the shell process of the selected container",
                            0,
                        ))
                        .unwrap();
                }
                ResponseEvent::Cancelled
            }
        } else {
            ResponseEvent::Handled
        }
    }

    fn process_disconnection(&mut self) {
        // pass
    }

    fn process_event(&mut self, event: TuiEvent) -> ResponseEvent {
        let TuiEvent::Key(key) = event;

        match key.code {
            KeyCode::Char(input) => self.bridge.send(get_bytes(input, key.modifiers)),
            KeyCode::Esc => self.bridge.send(vec![27]),
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
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(1), Constraint::Fill(1)])
            .split(area);

        self.header.draw(frame, layout[0]);

        if !self.bridge.is_running() {
            return;
        }

        if self.size.0 != layout[1].width || self.size.1 != layout[1].height {
            if let Ok(mut parser) = self.parser.write() {
                parser.set_size(layout[1].height, layout[1].width);
                self.bridge.set_terminal_size(layout[1].width, layout[1].height);
                self.size = (layout[1].width, layout[1].height);
            }
        }

        if let Ok(parser) = self.parser.read() {
            let screen = parser.screen();
            let pseudo_term = PseudoTerminal::new(screen);
            frame.render_widget(pseudo_term, layout[1]);
        }
    }
}

impl Drop for ShellView {
    fn drop(&mut self) {
        self.bridge.stop();
    }
}

fn get_bytes(input: char, modifiers: KeyModifiers) -> Vec<u8> {
    if modifiers == KeyModifiers::CONTROL {
        let mut result = input.to_ascii_uppercase().to_string().into_bytes();
        result[0] = result[0].saturating_sub(64);
        result
    } else {
        input.to_string().into_bytes()
    }
}
