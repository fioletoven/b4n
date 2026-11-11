use b4n_common::{DEFAULT_ERROR_DURATION, NotificationSink};
use b4n_config::keys::KeyCommand;
use b4n_kube::{ContainerRef, NODES, PODS, ResourceTag};
use b4n_tasks::commands::CommandResult;
use b4n_tui::utils::center;
use b4n_tui::widgets::{ActionItem, ActionsListBuilder, Button, Dialog, Spinner};
use b4n_tui::{MouseEventKind, ResponseEvent, Responsive, TuiEvent};
use crossterm::event::{KeyCode, KeyModifiers};
use kube::{Client, api::TerminalSize};
use ratatui::layout::{Constraint, Position};
use ratatui::text::{Line, Span};
use ratatui::{Frame, layout::Rect};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, RwLock};
use tokio::runtime::Handle;
use tokio_util::sync::CancellationToken;
use tui_term::widget::Cursor;
use tui_term::{vt100, widget::PseudoTerminal};

use crate::core::{SharedAppData, SharedAppDataExt, SharedBgWorker};
use crate::ui::presentation::ScreenSelection;
use crate::ui::views::shell::keys::{encode_key, encode_mouse};
use crate::ui::views::shell::kube::bridge::ShellBridge;
use crate::ui::views::shell::terminal::{CursorShapeTracker, FrameExt, RectExt, draw_scrollbar};
use crate::ui::views::{ESCAPE_SEQUENCE_TIMEOUT, EscapeSequenceTracker, ScreenExt, get_layout_with_header, transfer};
use crate::ui::widgets::{CommandPalette, FileSelector};
use crate::ui::{presentation::ContentHeader, views::View};

const DEFAULT_SHELL: &str = "bash";
const FALLBACK_SHELL: &str = "sh";
const SCROLLBACK_LEN: usize = 1_000;

/// Mode of the shell view.
#[derive(Clone, Copy, PartialEq)]
pub enum ShellMode {
    Shell,
    Attach,
    Node,
}

/// Pod's shell view.
pub struct ShellView {
    app_data: SharedAppData,
    header: ContentHeader,
    bridge: ShellBridge,
    parser: Arc<RwLock<vt100::Parser>>,
    size: TerminalSize,
    client: Client,
    pod: ContainerRef,
    command_id: Option<String>,
    command_failed: bool,
    cancellation: Option<CancellationToken>,
    scrollback_rows: usize,
    last_mouse_click: Option<Position>,
    modal: Dialog,
    command_palette: CommandPalette,
    file_picker: FileSelector,
    selection: ScreenSelection,
    area: Rect,
    alt_mode: EscapeSequenceTracker,
    clipboard_text: Option<String>,
    mode: ShellMode,
    is_app_mode: bool,
    is_mouse_enabled: bool,
    own_cursor: bool,
    cursor_shape: CursorShapeTracker,
    spinner: Spinner,
    footer_tx: NotificationSink,
}

impl ShellView {
    /// Creates new [`ShellView`] instance.
    pub fn new(
        app_data: SharedAppData,
        worker: SharedBgWorker,
        client: Client,
        pod: ContainerRef,
        mode: ShellMode,
        footer_tx: NotificationSink,
        workspace: Rect,
    ) -> Self {
        let header = Self::create_header(&app_data, &pod, mode);
        let runtime = worker.borrow().runtime_handle().clone();
        let area = get_layout_with_header(workspace)[1];
        let scrollback = app_data.borrow().config.terminal.scrollback_lines.unwrap_or(SCROLLBACK_LEN);
        let bridge = Self::create_bridge(runtime, &client, &pod, mode, area, scrollback);
        let parser = bridge.get_parser();
        let selection = ScreenSelection::default().with_color(app_data.borrow().theme.colors.shell.select);
        let own_cursor = app_data.borrow().config.terminal.system_cursor.is_none_or(|c| !c);
        let file_picker = FileSelector::new(Rc::clone(&app_data), worker, 65, PathBuf::from("."));
        let use_plain_symbols = app_data.use_plain_symbols();

        app_data.disable_command(KeyCommand::ApplicationExit, true);
        app_data.disable_command(KeyCommand::MouseSupportToggle, true);

        let result = Self {
            app_data,
            header,
            bridge,
            parser,
            size: area.to_terminal_size(),
            client,
            pod,
            command_id: None,
            command_failed: false,
            cancellation: None,
            scrollback_rows: 0,
            last_mouse_click: None,
            modal: Dialog::default(),
            command_palette: CommandPalette::default(),
            file_picker,
            selection,
            area,
            alt_mode: EscapeSequenceTracker::new(ESCAPE_SEQUENCE_TIMEOUT),
            clipboard_text: None,
            mode,
            is_app_mode: false,
            is_mouse_enabled: false,
            own_cursor,
            cursor_shape: CursorShapeTracker::new(!own_cursor),
            spinner: Spinner::new(use_plain_symbols),
            footer_tx,
        };

        if mode == ShellMode::Node {
            result.set_pending_hint();
        } else {
            result.update_hint();
        }

        result
    }

    /// Marks shell view as waiting for a task.
    /// - `command_id` is a task id that is creating the pod
    /// - `cancellation` is a token that should be cancelled to delete that pod
    pub fn wait_for_pod(mut self, command_id: String, cancellation: CancellationToken) -> Self {
        self.command_id = Some(command_id);
        self.command_failed = false;
        self.cancellation = Some(cancellation);
        self
    }

    #[inline]
    fn is_attach(&self) -> bool {
        self.mode == ShellMode::Attach
    }

    fn create_header(app_data: &SharedAppData, pod: &ContainerRef, mode: ShellMode) -> ContentHeader {
        let is_attach = mode == ShellMode::Attach;

        let mut header = ContentHeader::new(Rc::clone(app_data), false);
        if app_data.has_nerd_font() {
            header.set_title(if is_attach { " attach" } else { " shell" });
        } else {
            header.set_title(if is_attach { "attach" } else { "shell" });
        }

        if mode == ShellMode::Node {
            header.set_data(pod.namespace.clone(), NODES.into(), Some(pod.name.clone()), None);
        } else {
            header.set_data(
                pod.namespace.clone(),
                PODS.into(),
                Some(pod.name.clone()),
                pod.container.clone(),
            );
        }

        header
    }

    fn create_bridge(
        runtime: Handle,
        client: &Client,
        pod: &ContainerRef,
        mode: ShellMode,
        area: Rect,
        scrollback: usize,
    ) -> ShellBridge {
        let mut bridge = ShellBridge::new(runtime, area, scrollback, mode == ShellMode::Attach);
        if mode != ShellMode::Node {
            bridge.start(client.clone(), pod.clone(), DEFAULT_SHELL, area.to_terminal_size());
        }

        bridge
    }

    fn show_mouse_menu(&mut self, x: u16, y: u16) -> ResponseEvent {
        if !self.app_data.borrow().is_connected() {
            return ResponseEvent::Handled;
        }

        let mut builder = ActionsListBuilder::default().with_menu_action(ActionItem::menu(2, '󰆒', "paste", "paste"));
        if self.mode != ShellMode::Node {
            builder = builder
                .with_menu_action(ActionItem::menu(5, '', "download files", "download"))
                .with_menu_action(ActionItem::menu(5, '', "upload files", "upload"));
        }

        if self.is_attach() {
            builder.add_menu_action(ActionItem::menu(100, '󰕍', "back", "detach"));
        } else {
            builder.add_menu_action(ActionItem::menu(100, '', "detach shell", "detach"));
        }

        let is_mouse_allowed = self.bridge.is_mouse_enabled().unwrap_or(self.is_attach());
        let is_selected = self.selection.sorted().is_some();

        if !self.is_mouse_enabled {
            let copy = if is_selected { "selection" } else { "all" };
            builder.add_menu_action(ActionItem::menu(1, '󰆏', format!("copy ␝{copy}␝"), "copy"));
        }

        if self.is_attach() && self.bridge.is_application_mode().is_none() {
            if self.is_app_mode {
                builder.add_menu_action(ActionItem::menu(3, '', "normal cursor key mode", "app_mode"));
            } else {
                builder.add_menu_action(ActionItem::menu(3, '', "app. cursor key mode", "app_mode"));
            }
        }

        if is_mouse_allowed {
            if self.is_mouse_enabled {
                builder.add_menu_action(ActionItem::menu(4, '󰍾', "release mouse", "mouse_off"));
            } else {
                builder.add_menu_action(ActionItem::menu(4, '󰍽', "capture mouse", "mouse_on"));
            }
        }

        let actions = builder.build(None, None);
        let width = u16::try_from(actions.max_item_len() + 4).unwrap_or(u16::MAX).max(22);

        self.command_palette =
            CommandPalette::new(Rc::clone(&self.app_data), actions, width).to_mouse_menu(self.app_data.has_nerd_font());
        self.command_palette.show_at((x.saturating_sub(3), y).into());

        ResponseEvent::Handled
    }

    fn process_command_palette_event(&mut self, event: &TuiEvent) -> ResponseEvent {
        let response = self.command_palette.process_event(event);
        if let ResponseEvent::Action(action) = response {
            return match action {
                "paste" => {
                    self.last_mouse_click = event.position();
                    self.insert_from_clipboard()
                },
                "copy" => self.copy_to_clipboard(),
                "app_mode" => self.toggle_app_mode(),
                "mouse_on" => self.enable_mouse(true),
                "mouse_off" => self.enable_mouse(false),
                "download" => {
                    self.last_mouse_click = event.position();
                    self.ask_transfer_file(true)
                },
                "upload" => {
                    self.last_mouse_click = event.position();
                    self.ask_transfer_file(false)
                },
                "detach" => {
                    if self.mode == ShellMode::Shell {
                        self.last_mouse_click = event.position();
                        self.ask_close_shell_forcibly()
                    } else {
                        ResponseEvent::Cancelled
                    }
                },
                _ => response,
            };
        }

        response
    }

    fn copy_to_clipboard(&mut self) -> ResponseEvent {
        if let Ok(parser) = self.parser.read() {
            parser
                .screen()
                .copy_to_clipboard(&mut self.app_data, &mut self.selection, &self.footer_tx);
        }

        self.selection.reset();
        ResponseEvent::Handled
    }

    /// Inserts clipboard text to the current shell session.\
    /// **Note** that it displays a confirmation dialog instead if the clipboard text contains multiple lines.
    fn insert_from_clipboard(&mut self) -> ResponseEvent {
        if !self.bridge.is_running() {
            return ResponseEvent::Handled;
        }

        let clipboard = self.app_data.borrow().get_clipboard();
        if let Some(text) = clipboard.and_then(|c| c.borrow_mut().get_text().ok()) {
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
             into the terminal, it may result in the unexpected execution of commands.\n\
             Do you wish to continue?"
                .to_owned(),
            vec![
                Button::new(
                    "Paste Anyway",
                    ResponseEvent::Action("paste"),
                    colors.modal.btn_accent.clone(),
                ),
                Button::new("Cancel", ResponseEvent::Action("cancel"), colors.modal.btn_cancel.clone()),
            ],
        )
        .with_width(65)
        .with_colors(colors.modal.text)
        .with_highlighted_position(self.last_mouse_click.take())
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
                Button::new("Close Anyway", ResponseEvent::Cancelled, colors.modal.btn_delete.clone()),
                Button::new("Cancel", ResponseEvent::Action("cancel"), colors.modal.btn_cancel.clone()),
            ],
        )
        .with_width(65)
        .with_colors(colors.modal.text)
        .with_highlighted_position(self.last_mouse_click.take())
    }

    /// Shows transfer file dialog.
    fn ask_transfer_file(&mut self, is_download: bool) -> ResponseEvent {
        if let Some(container) = &self.pod.container {
            let tags = [ResourceTag::Container(container.to_owned(), self.pod.kind, None)];
            self.modal = transfer::new_transfer_dialog(is_download, &self.app_data, &tags, self.last_mouse_click.take());
            self.modal.show();
        }

        ResponseEvent::Handled
    }

    fn show_file_picker(&mut self, remote: bool) {
        let is_download = self.modal.checkbox(0).is_some_and(|cb| cb.is_checked);
        let source = if remote {
            if let Some(context) = transfer::get_transfer_dialog_context(&self.modal) {
                let mut resource = self.pod.clone();
                resource.container = Some(context.container);
                Some((self.client.clone(), resource))
            } else {
                return;
            }
        } else {
            None
        };
        let current_path = transfer::get_path_for_file_picker(&self.modal, remote);
        self.file_picker.set_current_path(current_path);
        self.file_picker.set_remote(source);
        self.file_picker.set_dir_selection(true, is_download != remote);
        self.file_picker.reset();
        self.file_picker.show();
    }

    fn set_scrollback(&mut self, offset: u16, is_up: bool) -> ResponseEvent {
        if is_up {
            self.scrollback_rows = self.scrollback_rows.saturating_add(usize::from(offset));
        } else {
            self.scrollback_rows = self.scrollback_rows.saturating_sub(usize::from(offset));
        }

        self.update_scrollback();
        ResponseEvent::Handled
    }

    fn reset_scrollback(&mut self, is_up: bool) -> ResponseEvent {
        if is_up {
            self.scrollback_rows = usize::MAX;
        } else {
            self.scrollback_rows = 0;
        }

        self.update_scrollback();
        ResponseEvent::Handled
    }

    fn update_scrollback(&mut self) {
        if let Ok(mut parser) = self.parser.write() {
            parser.screen_mut().set_scrollback(self.scrollback_rows);
            self.scrollback_rows = parser.screen().scrollback();
        }
    }

    fn toggle_app_mode(&mut self) -> ResponseEvent {
        self.is_app_mode = !self.is_app_mode;
        ResponseEvent::Handled
    }

    fn enable_mouse(&mut self, is_enabled: bool) -> ResponseEvent {
        self.is_mouse_enabled = is_enabled;
        if is_enabled {
            let icon = self.app_data.borrow().config.symbols.hint.right;
            self.footer_tx
                .show_hint(format!("{icon}Double right-click to open mouse menu"));
        } else {
            self.update_hint();
        }

        ResponseEvent::Handled
    }

    fn hide_cursor(&self) -> bool {
        self.bridge.is_finished() || self.command_palette.is_visible || self.modal.is_visible
    }

    fn process_widget_event(&mut self, event: &TuiEvent) -> Option<ResponseEvent> {
        if self.file_picker.is_visible {
            if self.modal.is_visible && self.file_picker.process_event(event) == ResponseEvent::Accepted {
                transfer::update_transfer_dialog_paths(&mut self.modal, &self.file_picker);
            }

            return Some(ResponseEvent::Handled);
        }

        if self.command_palette.is_visible {
            let result = self.process_command_palette_event(event);
            if result != ResponseEvent::NotHandled
                || (event.is_mouse(MouseEventKind::LeftClick) && self.selection.sorted().is_some())
            {
                return Some(result);
            }
        }

        if self.modal.is_visible {
            let response = self.modal.process_event(event);
            return Some(match response {
                ResponseEvent::Action(action) => match action {
                    "paste" => {
                        if let Some(text) = self.clipboard_text.take() {
                            self.selection.reset();
                            self.bridge.send(text.into_bytes());
                        }
                        ResponseEvent::Handled
                    },
                    "select_file" | "select_remote_file" => {
                        self.show_file_picker(action == "select_remote_file");
                        ResponseEvent::Handled
                    },
                    "transfer_file" => {
                        if let Some(context) = transfer::get_transfer_dialog_context(&self.modal) {
                            ResponseEvent::TransferFile(self.pod.clone(), context)
                        } else {
                            ResponseEvent::Handled
                        }
                    },
                    _ => ResponseEvent::Handled,
                },
                ResponseEvent::Cancelled => ResponseEvent::Cancelled,
                _ => ResponseEvent::Handled,
            });
        }

        None
    }

    fn process_shell_event(&mut self, event: &TuiEvent) -> ResponseEvent {
        let TuiEvent::Key(key) = event else {
            return ResponseEvent::Handled;
        };

        if key.modifiers == KeyModifiers::CONTROL {
            match key.code {
                KeyCode::Home => return self.reset_scrollback(true),
                KeyCode::Up => return self.set_scrollback(1, true),
                KeyCode::PageUp => return self.set_scrollback(self.size.height, true),
                KeyCode::Down => return self.set_scrollback(1, false),
                KeyCode::PageDown => return self.set_scrollback(self.size.height, false),
                KeyCode::End => return self.reset_scrollback(false),
                _ => (),
            }
        }

        let is_app_mode = self.bridge.is_application_mode().unwrap_or(self.is_app_mode);
        if let Some(bytes) = encode_key(key.code, key.modifiers, is_app_mode) {
            self.bridge.send(bytes);

            if self.scrollback_rows > 0 {
                self.reset_scrollback(false);
            }
        }

        ResponseEvent::Handled
    }

    fn draw_empty(&mut self, frame: &mut Frame<'_>, area: Rect) {
        let colors = &self.app_data.borrow().theme.colors;
        let line = Line::default()
            .spans([
                Span::raw(self.spinner.tick().to_string()),
                " waiting for node shell pod to become ready…".into(),
            ])
            .style(&colors.text);
        let area = center(area, Constraint::Length(line.width() as u16), Constraint::Length(4));
        frame.render_widget(line, area);
    }

    fn update_hint(&self) {
        let key = self.app_data.get_key_name(KeyCommand::ShellEscape).to_ascii_uppercase();
        let icon = self.app_data.borrow().config.symbols.hint.right;
        if self.alt_mode.is_active() {
            if self.mode == ShellMode::Node {
                self.footer_tx
                    .show_hint(format!("{icon}Escape mode: press ␝{key}␝ to close node shell"));
            } else {
                let is_attach = self.mode == ShellMode::Attach;
                let action = if is_attach { "close attach view" } else { "detach shell" };
                let transfer_to = self.app_data.get_key_name(KeyCommand::TransferTo).to_ascii_uppercase();
                let transfer_from = self.app_data.get_key_name(KeyCommand::TransferFrom).to_ascii_uppercase();
                self.footer_tx.show_hint(format!(
                    "{icon}Escape mode: press ␝{key}␝ to {action}, ␝{transfer_to}␝ or ␝{transfer_from}␝ for file transfer"
                ));
            }
        } else {
            self.footer_tx
                .show_hint(format!("{icon}Double press ␝{key}␝ to activate escape mode"));
        }
    }

    fn set_pending_hint(&self) {
        let key = self.app_data.get_key_name(KeyCommand::ShellEscape).to_ascii_uppercase();
        let icon = self.app_data.borrow().config.symbols.hint.right;
        self.footer_tx
            .show_hint(format!("{icon}Press ␝{key}␝ to cancel node shell creation"));
    }
}

impl View for ShellView {
    fn command_id(&self) -> Option<&str> {
        self.command_id.as_deref()
    }

    fn process_command_result(&mut self, result: CommandResult) {
        if let CommandResult::CreatedNodeShell(result) = result {
            match result {
                Ok(pod) => {
                    let command = [
                        "nsenter", "--target", "1", "--mount", "--uts", "--ipc", "--net", "--pid", "--", "sh",
                    ];
                    self.pod = pod;
                    self.bridge.start_command(
                        self.client.clone(),
                        self.pod.clone(),
                        command.into_iter().map(str::to_owned).collect(),
                        self.area.to_terminal_size(),
                    );
                },
                Err(error) => {
                    self.footer_tx.show_error(error.to_string(), DEFAULT_ERROR_DURATION);
                    self.command_failed = true;
                },
            }

            self.update_hint();
        }
    }

    fn process_tick(&mut self) -> ResponseEvent {
        if self.command_failed {
            return ResponseEvent::Cancelled;
        }

        self.cursor_shape.sync_shape(self.bridge.cursor_shape());

        if self.bridge.is_finished() {
            // we try to fall back to 'sh' if ShellBridge has an error and was initially started as 'bash'
            if !self.is_attach() && self.bridge.has_error() && self.bridge.shell().is_some_and(|s| s == DEFAULT_SHELL) {
                let size = self.area.to_terminal_size();
                self.bridge.start(self.client.clone(), self.pod.clone(), FALLBACK_SHELL, size);
            } else {
                if self.bridge.has_error() {
                    let kind = if self.is_attach() { "main" } else { "shell" };
                    self.footer_tx.show_error(
                        format!("Unable to attach to the {kind} process of the selected container"),
                        DEFAULT_ERROR_DURATION,
                    );
                }
                return ResponseEvent::Cancelled;
            }
        } else if self.is_mouse_enabled && self.bridge.is_application_mode().is_some_and(|m| !m) {
            return self.enable_mouse(false);
        }

        if let Some(event) = self.alt_mode.get_recorded_event(false) {
            self.process_shell_event(&event);
        }

        ResponseEvent::Handled
    }

    fn process_disconnection(&mut self) {
        if let Some(cancellation) = &self.cancellation {
            cancellation.cancel();
            self.command_failed = true;
        }
    }

    fn process_event(&mut self, event: &TuiEvent) -> ResponseEvent {
        if let Some(value) = self.process_widget_event(event) {
            return value;
        }

        if self.alt_mode.consume_active() {
            self.update_hint();
            if self.app_data.has_binding(event, KeyCommand::ShellEscape) {
                return if self.mode == ShellMode::Shell {
                    self.ask_close_shell_forcibly()
                } else {
                    ResponseEvent::Cancelled
                };
            }

            if self.mode != ShellMode::Node {
                if self.app_data.has_binding(event, KeyCommand::TransferTo) {
                    return self.ask_transfer_file(false);
                }

                if self.app_data.has_binding(event, KeyCommand::TransferFrom) {
                    return self.ask_transfer_file(true);
                }
            }
        }

        if self.app_data.has_binding(event, KeyCommand::ShellEscape) {
            if !self.bridge.is_running() {
                return ResponseEvent::Cancelled;
            }

            if let Some(prev_event) = self.alt_mode.record_event(event) {
                return self.process_shell_event(&prev_event);
            }

            self.update_hint();
            return ResponseEvent::Handled;
        }

        if !self.is_mouse_enabled
            && let Ok(parser) = self.parser.read()
        {
            self.selection.process_screen_event(event, parser.screen(), self.area);
        }

        if let TuiEvent::Mouse(mouse) = event {
            if self.is_mouse_enabled {
                if mouse.kind == MouseEventKind::RightDoubleClick {
                    return self.show_mouse_menu(mouse.column, mouse.row);
                }
                if self.is_mouse_enabled
                    && let Some(bytes) = encode_mouse(mouse.kind, mouse.column, mouse.row, self.area, mouse.modifiers)
                {
                    self.bridge.send(bytes);
                }

                return ResponseEvent::Handled;
            }

            return match mouse.kind {
                MouseEventKind::ScrollUp => self.set_scrollback(1, true),
                MouseEventKind::ScrollDown => self.set_scrollback(1, false),
                MouseEventKind::RightClick => self.show_mouse_menu(mouse.column, mouse.row),
                _ => ResponseEvent::NotHandled,
            };
        }

        if let Some(prev_event) = self.alt_mode.get_recorded_event(true) {
            self.process_shell_event(&prev_event);
        }

        self.process_shell_event(event)
    }

    fn draw(&mut self, frame: &mut Frame<'_>, area: Rect, has_focus: bool) {
        let layout = get_layout_with_header(area);
        self.area = layout[1];

        self.header.draw(frame, layout[0]);

        if self.bridge.is_running() {
            if (self.size.width != layout[1].width || self.size.height != layout[1].height)
                && let Ok(mut parser) = self.parser.write()
            {
                parser.screen_mut().set_size(layout[1].height, layout[1].width);
                self.bridge.set_terminal_size(layout[1].width, layout[1].height);
                self.size = layout[1].to_terminal_size();
            }

            if let Ok(mut parser) = self.parser.write() {
                let screen = parser.screen_mut();
                let cursor = Cursor::default().visibility(self.own_cursor && has_focus && !self.hide_cursor());
                let pseudo_term = PseudoTerminal::new(&*screen).cursor(cursor);

                frame.render_widget(pseudo_term, layout[1]);

                if !self.own_cursor && has_focus && !self.hide_cursor() && !screen.hide_cursor() {
                    frame.set_cursor_screen_position(screen, layout[1]);
                }

                draw_scrollbar(frame, layout[1], &self.app_data, screen);
            }

            if !self.is_mouse_enabled {
                frame.render_widget(&self.selection, layout[1]);
            }
        } else if self.mode == ShellMode::Node {
            self.draw_empty(frame, layout[1]);
        }

        self.command_palette.draw(frame, frame.area());
        self.modal.draw(frame, frame.area());
        self.file_picker.draw(frame, frame.area());
    }
}

impl Drop for ShellView {
    fn drop(&mut self) {
        if let Some(cancellation) = &self.cancellation {
            cancellation.cancel();
        }

        self.bridge.stop();
        self.app_data.disable_command(KeyCommand::ApplicationExit, false);
        self.app_data.disable_command(KeyCommand::MouseSupportToggle, false);
        self.footer_tx.hide_hint();
    }
}
