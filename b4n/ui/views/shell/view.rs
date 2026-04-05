use b4n_common::{DEFAULT_ERROR_DURATION, NotificationSink};
use b4n_config::keys::KeyCommand;
use b4n_kube::client::KubernetesClient;
use b4n_kube::{ContainerRef, Namespace, PODS};
use b4n_tui::widgets::{ActionItem, ActionsListBuilder, Button, Dialog};
use b4n_tui::{MouseEventKind, ResponseEvent, Responsive, TuiEvent};
use crossterm::event::{KeyCode, KeyModifiers};
use kube::{Client, api::TerminalSize};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use std::rc::Rc;
use std::sync::{Arc, RwLock};
use std::time::Instant;
use tokio::runtime::Handle;
use tui_term::{vt100, widget::PseudoTerminal};

use crate::core::{SharedAppData, SharedAppDataExt};
use crate::ui::presentation::ScreenSelection;
use crate::ui::widgets::CommandPalette;
use crate::ui::{presentation::ContentHeader, views::View};

use super::bridge::ShellBridge;

const DEFAULT_SHELL: &str = "bash";
const FALLBACK_SHELL: &str = "sh";
const DEFAULT_SIZE: TerminalSize = TerminalSize { width: 80, height: 24 };
const SCROLLBACK_LEN: usize = 1_000;

/// Pod's shell view.
pub struct ShellView {
    app_data: SharedAppData,
    header: ContentHeader,
    bridge: ShellBridge,
    parser: Arc<RwLock<vt100::Parser>>,
    size: TerminalSize,
    client: Client,
    pod: ContainerRef,
    scrollback_rows: usize,
    modal: Dialog,
    command_palette: CommandPalette,
    selection: ScreenSelection,
    area: Rect,
    esc_count: u8,
    esc_time: Instant,
    clipboard_text: Option<String>,
    footer_tx: NotificationSink,
}

impl ShellView {
    /// Creates new [`ShellView`] instance.
    pub fn new(
        runtime: Handle,
        app_data: SharedAppData,
        client: &KubernetesClient,
        pod_name: String,
        pod_namespace: Namespace,
        pod_container: Option<String>,
        footer_tx: NotificationSink,
    ) -> Self {
        let pod = ContainerRef::simple(pod_name.clone(), pod_namespace.clone(), pod_container.clone());
        let mut header = ContentHeader::new(Rc::clone(&app_data), false);
        header.set_title(" shell");
        header.set_data(pod_namespace, PODS.into(), Some(pod_name), pod_container);

        let selection = ScreenSelection::default().with_color(app_data.borrow().theme.colors.shell.select);
        let parser = Arc::new(RwLock::new(vt100::Parser::new(
            DEFAULT_SIZE.height,
            DEFAULT_SIZE.width,
            SCROLLBACK_LEN,
        )));
        let mut bridge = ShellBridge::new(runtime, parser.clone());
        bridge.start(client.get_client(), pod.clone(), DEFAULT_SHELL);

        app_data.disable_command(KeyCommand::ApplicationExit, true);
        app_data.disable_command(KeyCommand::MouseSupportToggle, true);

        let key = app_data.get_key_name(KeyCommand::ShellEscape).to_ascii_uppercase();
        footer_tx.show_hint(format!(" Press ␝{key}␝ rapidly ␝3␝ times to detach shell"));

        Self {
            app_data,
            header,
            bridge,
            parser,
            size: DEFAULT_SIZE,
            client: client.get_client(),
            pod,
            scrollback_rows: 0,
            modal: Dialog::default(),
            command_palette: CommandPalette::default(),
            selection,
            area: Rect::default(),
            esc_count: 0,
            esc_time: Instant::now(),
            clipboard_text: None,
            footer_tx,
        }
    }

    fn show_mouse_menu(&mut self, x: u16, y: u16) -> ResponseEvent {
        if !self.app_data.borrow().is_connected() {
            return ResponseEvent::Handled;
        }

        let is_selected = self.selection.sorted().is_some();
        let copy = if is_selected { "selected" } else { "all" };
        let builder = ActionsListBuilder::default()
            .with_menu_action(ActionItem::menu(1, &format!("󰆏 copy ␝{copy}␝"), "copy"))
            .with_menu_action(ActionItem::menu(2, "󰆒 paste", "paste"))
            .with_menu_action(ActionItem::menu(100, " detach shell", "detach"));

        self.command_palette = CommandPalette::new(Rc::clone(&self.app_data), builder.build(None), 22).to_mouse_menu();
        self.command_palette.show_at((x.saturating_sub(3), y).into());

        ResponseEvent::Handled
    }

    fn process_command_palette_event(&mut self, event: &TuiEvent) -> ResponseEvent {
        let response = self.command_palette.process_event(event);
        if let ResponseEvent::Action(action) = response {
            return match action {
                "paste" => self.insert_from_clipboard(),
                "copy" => self.copy_to_clipboard(),
                "detach" => self.ask_close_shell_forcibly(),
                _ => response,
            };
        }

        response
    }

    /// Inserts clipboard text to the current shell session.\
    /// **Note** that it displays a confirmation dialog instead if the clipboard text contains multiple lines.
    fn insert_from_clipboard(&mut self) -> ResponseEvent {
        let text = self.app_data.borrow_mut().clipboard.as_mut().and_then(|c| c.get_text().ok());
        if let Some(text) = text {
            if text.contains('\n') {
                self.clipboard_text = Some(text.replace("\r\n", "\n"));
                self.ask_insert_from_clipboard();
            } else {
                self.selection.reset();
                self.bridge.send(text.into_bytes());
            }
        }

        ResponseEvent::Handled
    }

    fn copy_to_clipboard(&mut self) -> ResponseEvent {
        if let Ok(parser) = self.parser.read() {
            if let Some((start, end)) = self.selection.sorted() {
                let text = parser.screen().contents_between(start.y, start.x, end.y, end.x + 1);
                self.app_data
                    .copy_to_clipboard(text, &self.footer_tx, || "Selected text copied to clipboard");
            } else {
                let text = parser.screen().contents();
                self.app_data
                    .copy_to_clipboard(text, &self.footer_tx, || "Whole screen copied to clipboard");
            }
        }

        self.selection.reset();
        ResponseEvent::Handled
    }

    /// Displays a confirmation dialog to paste multiline clipboard text.
    fn ask_insert_from_clipboard(&mut self) {
        if self.bridge.is_running() && self.clipboard_text.is_some() {
            self.modal = self.new_insert_clipboard_text_dialog();
            self.modal.show();
        }
    }

    /// Creates new insert multiline clipboard text dialog.
    fn new_insert_clipboard_text_dialog(&mut self) -> Dialog {
        let colors = &self.app_data.borrow().theme.colors;

        Dialog::new(
            "You are about to paste text that contains multiple lines. If you paste this text \
             into your shell, it may result in the unexpected execution of commands.\n\
             Do you wish to continue?"
                .to_owned(),
            vec![
                Button::new("Paste Anyway", ResponseEvent::Action("paste"), &colors.modal.btn_accent),
                Button::new("Cancel", ResponseEvent::Action("cancel"), &colors.modal.btn_cancel),
            ],
            65,
            colors.modal.text,
        )
    }

    /// Displays a confirmation dialog to forcibly close the shell view.
    fn ask_close_shell_forcibly(&mut self) -> ResponseEvent {
        if self.bridge.is_running() {
            self.modal = self.new_close_dialog();
            self.modal.show();
        }

        ResponseEvent::Handled
    }

    /// Creates new close dialog.
    fn new_close_dialog(&mut self) -> Dialog {
        let colors = &self.app_data.borrow().theme.colors;

        Dialog::new(
            "You are about to close the shell view without terminating the running shell process. \
             It will keep running in the background until you stop it manually. Type 'exit' to close it gracefully."
                .to_owned(),
            vec![
                Button::new("Close Anyway", ResponseEvent::Cancelled, &colors.modal.btn_delete),
                Button::new("Cancel", ResponseEvent::Action("cancel"), &colors.modal.btn_cancel),
            ],
            65,
            colors.modal.text,
        )
    }

    /// Checks if `ESC` key was pressed quickly `x` times.
    fn is_esc_key_pressed_times(&mut self, times: u8) -> bool {
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

    fn set_scrollback(&mut self, offset: u16, is_up: bool) -> ResponseEvent {
        if is_up {
            self.scrollback_rows = self.scrollback_rows.saturating_add(usize::from(offset));
        } else {
            self.scrollback_rows = self.scrollback_rows.saturating_sub(usize::from(offset));
        }

        if let Ok(mut parser) = self.parser.write() {
            parser.screen_mut().set_scrollback(self.scrollback_rows);
            self.scrollback_rows = parser.screen().scrollback();
        }

        ResponseEvent::Handled
    }

    fn reset_scrollback(&mut self) {
        self.scrollback_rows = 0;
        if let Ok(mut parser) = self.parser.write() {
            parser.screen_mut().set_scrollback(0);
        }
    }
}

impl View for ShellView {
    fn process_tick(&mut self) -> ResponseEvent {
        if self.bridge.is_finished() {
            // we try to fall back to 'sh' if ShellBridge has an error and was initially started as 'bash'
            if self.bridge.has_error() && self.bridge.shell().is_some_and(|s| s == DEFAULT_SHELL) {
                self.bridge.start(self.client.clone(), self.pod.clone(), FALLBACK_SHELL);
                self.size = DEFAULT_SIZE;
                ResponseEvent::Handled
            } else {
                if self.bridge.has_error() {
                    self.footer_tx.show_error(
                        "Unable to attach to the shell process of the selected container",
                        DEFAULT_ERROR_DURATION,
                    );
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

    fn process_event(&mut self, event: &TuiEvent) -> ResponseEvent {
        if self.command_palette.is_visible {
            let result = self.process_command_palette_event(event);
            if result != ResponseEvent::NotHandled {
                return result;
            }
        }

        if self.modal.is_visible {
            return self.modal.process_event(event).when_action_then("paste", || {
                if let Some(text) = self.clipboard_text.take() {
                    self.selection.reset();
                    self.bridge.send(text.into_bytes());
                }
                ResponseEvent::Handled
            });
        }

        if self.app_data.has_binding(event, KeyCommand::ShellEscape) && self.is_esc_key_pressed_times(3) {
            return self.ask_close_shell_forcibly();
        }

        if let Ok(parser) = self.parser.read() {
            self.selection.process_event(event, parser.screen(), self.area);
        }

        if let TuiEvent::Mouse(mouse) = event {
            return match mouse.kind {
                MouseEventKind::ScrollUp => self.set_scrollback(1, true),
                MouseEventKind::ScrollDown => self.set_scrollback(1, false),
                MouseEventKind::RightClick => self.show_mouse_menu(mouse.column, mouse.row),
                _ => ResponseEvent::NotHandled,
            };
        }

        if let TuiEvent::Key(key) = event {
            if key.modifiers == KeyModifiers::CONTROL {
                match key.code {
                    KeyCode::Up => return self.set_scrollback(1, true),
                    KeyCode::PageUp => return self.set_scrollback(self.size.height, true),
                    KeyCode::Down => return self.set_scrollback(1, false),
                    KeyCode::PageDown => return self.set_scrollback(self.size.height, false),
                    _ => (),
                }
            }

            let mut key_processed = true;
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
                _ => key_processed = false,
            }

            if key_processed && self.scrollback_rows > 0 {
                self.reset_scrollback();
            }
        }

        ResponseEvent::Handled
    }

    fn draw(&mut self, frame: &mut Frame<'_>, area: Rect) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(1), Constraint::Fill(1)])
            .split(area);
        self.area = layout[1];

        self.header.draw(frame, layout[0]);

        if !self.bridge.is_running() {
            return;
        }

        if (self.size.width != layout[1].width || self.size.height != layout[1].height)
            && let Ok(mut parser) = self.parser.write()
        {
            parser.screen_mut().set_size(layout[1].height, layout[1].width);
            self.bridge.set_terminal_size(layout[1].width, layout[1].height);
            self.size = TerminalSize {
                width: layout[1].width,
                height: layout[1].height,
            };
        }

        if let Ok(parser) = self.parser.read() {
            let screen = parser.screen();
            let pseudo_term = PseudoTerminal::new(screen);
            frame.render_widget(pseudo_term, layout[1]);
        }

        frame.render_widget(&self.selection, layout[1]);
        self.command_palette.draw(frame, frame.area());
        self.modal.draw(frame, frame.area());
    }
}

impl Drop for ShellView {
    fn drop(&mut self) {
        self.bridge.stop();
        self.app_data.disable_command(KeyCommand::ApplicationExit, false);
        self.app_data.disable_command(KeyCommand::MouseSupportToggle, false);
        self.footer_tx.hide_hint();
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
