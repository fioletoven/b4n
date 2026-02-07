use b4n_common::{DEFAULT_MESSAGE_DURATION, IconKind, NotificationSink, sanitize_and_split};
use b4n_config::keys::{KeyCombination, KeyCommand};
use b4n_kube::utils::deserialize_kind;
use b4n_kube::{ResourceRef, SECRETS};
use b4n_tasks::commands::{
    CommandResult, ResourceYamlResult, SetNewResourceYamlOptions, SetResourceYamlAction, SetResourceYamlOptions,
};
use b4n_tui::widgets::{ActionItem, ActionsListBuilder, Button, CheckBox, Dialog};
use b4n_tui::{MouseEventKind, ResponseEvent, Responsive, TuiEvent};
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::Position;
use ratatui::{Frame, layout::Rect};
use std::rc::Rc;

use crate::core::{SharedAppData, SharedAppDataExt, SharedBgWorker};
use crate::ui::presentation::{Content, ContentViewer, StyleFallback};
use crate::ui::views::{View, yaml::YamlContent};
use crate::ui::widgets::{CommandPalette, Search};

/// YAML view.
pub struct YamlView {
    yaml: ContentViewer<YamlContent>,
    app_data: SharedAppData,
    worker: SharedBgWorker,
    is_hint_visible: bool,
    is_new: bool,
    is_edit: bool,
    is_secret: bool,
    is_decoded: bool,
    can_patch_status: bool,
    origin_kind: Option<String>,
    command_id: Option<String>,
    last_mouse_click: Option<Position>,
    search: Search,
    modal: Dialog,
    command_palette: CommandPalette,
    footer: NotificationSink,
    state: ViewState,
    copied_line: Option<String>,
}

impl YamlView {
    /// Creates new [`YamlView`] instance.
    pub fn new(
        app_data: SharedAppData,
        worker: SharedBgWorker,
        command_id: Option<String>,
        resource: ResourceRef,
        footer: NotificationSink,
        is_new: bool,
        workspace: Rect,
    ) -> Self {
        let select = app_data.borrow().theme.colors.syntax.yaml.select;
        let search = app_data.borrow().theme.colors.syntax.yaml.search;
        let is_secret = resource.kind.name() == SECRETS;
        let name = if is_new { None } else { resource.name };
        let area = ContentViewer::<YamlContent>::get_content_area(workspace);
        let yaml = ContentViewer::new(Rc::clone(&app_data), select, search, area).with_header(
            if is_new { "create new resource" } else { "YAML" },
            '',
            resource.namespace,
            resource.kind,
            name,
            None,
        );
        let search = Search::new(Rc::clone(&app_data), Some(Rc::clone(&worker)), 65);

        Self {
            yaml,
            app_data,
            worker,
            is_hint_visible: false,
            is_new,
            is_edit: false,
            is_secret,
            is_decoded: false,
            can_patch_status: false,
            origin_kind: None,
            command_id,
            last_mouse_click: None,
            search,
            modal: Dialog::default(),
            command_palette: CommandPalette::default(),
            footer,
            state: ViewState::Idle,
            copied_line: None,
        }
    }

    /// Marks YAML view to switch to edit when data is received.
    pub fn switch_to_edit(&mut self) {
        self.is_edit = true;
        self.state = ViewState::WaitingForEdit;
    }

    fn copy_to_clipboard(&mut self, is_current_line: bool) {
        if self.yaml.content().is_none() {
            return;
        }

        let text = if is_current_line && !self.yaml.has_selection() {
            let line = self.yaml.get_current_line().map(String::from).unwrap_or_default();
            self.copied_line = Some(line.clone());
            line
        } else {
            let range = self.yaml.get_selection();
            self.copied_line = None;
            self.yaml.content().map(|c| c.to_plain_text(range)).unwrap_or_default()
        };

        self.app_data.copy_to_clipboard(text, &self.footer, || {
            if self.yaml.has_selection() {
                "Selection copied to clipboard"
            } else if is_current_line {
                "Line copied to clipboard"
            } else {
                "YAML content copied to clipboard"
            }
        });
    }

    fn insert_from_clipboard(&mut self) {
        if !self.yaml.is_in_edit_mode() {
            return;
        }

        if let Some(clipboard) = &mut self.app_data.borrow_mut().clipboard
            && let Ok(text) = clipboard.get_text()
        {
            if self.copied_line.as_ref().is_some_and(|l| *l == text) {
                self.yaml.insert_text(vec![text, String::new()], true);
            } else {
                self.yaml.insert_text(sanitize_and_split(&text), false);
            }
        }
    }

    fn can_encode_decode(&self) -> bool {
        self.yaml.header.kind.as_str() == SECRETS && self.app_data.borrow().is_connected() && !self.yaml.is_modified()
    }

    fn show_command_palette(&mut self) {
        let mut builder = ActionsListBuilder::default()
            .with_back()
            .with_quit()
            .with_action(
                ActionItem::action("copy", "copy").with_description("copies YAML to clipboard"),
                Some(KeyCommand::ContentCopy),
            )
            .with_action(
                ActionItem::action("search", "search").with_description("searches YAML using the provided query"),
                Some(KeyCommand::SearchOpen),
            );
        if self.yaml.content().is_some_and(Content::is_editable) {
            builder.add_action(
                ActionItem::action("edit", "edit")
                    .with_description("switches to the edit mode")
                    .with_aliases(&["insert"]),
                Some(KeyCommand::YamlEdit),
            );
        }
        if self.can_encode_decode() {
            let action = if self.is_decoded { "encode" } else { "decode" };
            builder.add_action(
                ActionItem::action(action, "decode").with_description(&format!("{action}s the resource's data")),
                Some(KeyCommand::YamlDecode),
            );
        }

        let actions = builder.build(self.app_data.borrow().config.key_bindings.as_ref());
        self.command_palette =
            CommandPalette::new(Rc::clone(&self.app_data), actions, 65).with_highlighted_position(self.last_mouse_click.take());
        self.command_palette.show();
        self.footer.hide_hint();
    }

    fn show_mouse_menu(&mut self, x: u16, y: u16) {
        let mut size = 22;
        let mut builder = ActionsListBuilder::default();
        if self.yaml.is_in_edit_mode() {
            size = 17;
            builder = builder
                .with_menu_action(ActionItem::menu(1, "󰆏 copy", "copy_2"))
                .with_menu_action(ActionItem::menu(2, "󰆐 cut", "cut"))
                .with_menu_action(ActionItem::menu(3, "󰆒 paste", "paste"))
                .with_menu_action(ActionItem::menu(4, "󰕌 undo", "undo"))
                .with_menu_action(ActionItem::menu(5, "󰑎 redo", "redo"))
                .with_menu_action(ActionItem::menu(100, " close edit", "back"));
        } else {
            let copy = if self.yaml.has_selection() { "selection" } else { "all" };
            builder = builder
                .with_menu_action(ActionItem::back())
                .with_menu_action(ActionItem::command_palette())
                .with_menu_action(ActionItem::menu(1, &format!("󰆏 copy ␝{copy}␝"), "copy"))
                .with_menu_action(ActionItem::menu(2, " search", "search"));
            if self.yaml.content().is_some_and(Content::is_editable) {
                builder.add_menu_action(ActionItem::menu(4, " edit", "edit"));
            }
            if self.can_encode_decode() {
                let action = if self.is_decoded { " encode" } else { " decode" };
                builder.add_menu_action(ActionItem::menu(3, action, "decode"));
            }
        }

        self.command_palette = CommandPalette::new(Rc::clone(&self.app_data), builder.build(None), size).to_mouse_menu();
        self.command_palette.show_at((x.saturating_sub(3), y).into());
    }

    fn toggle_yaml_decode(&mut self) {
        if !self.app_data.borrow().is_connected() || self.yaml.is_modified() {
            return;
        }

        self.yaml.clear_selection();
        self.clear_search();
        self.command_id = self.worker.borrow_mut().get_yaml(
            self.yaml.header.name.as_deref().map(String::from).unwrap_or_default(),
            self.yaml.header.namespace.clone(),
            self.yaml.header.kind.clone(),
            !self.is_decoded,
        );
    }

    fn clear_search(&mut self) {
        self.yaml.search("", false);
        self.search.reset();
        self.update_search_count();
    }

    fn update_search_count(&mut self) {
        self.footer
            .set_text("900_yaml_search", self.yaml.get_footer_text(), IconKind::Default);
        self.search.set_matches(self.yaml.matches_count());
    }

    fn navigate_match(&mut self, forward: bool) {
        self.yaml.navigate_match(forward, None);
        self.footer
            .set_text("900_yaml_search", self.yaml.get_footer_text(), IconKind::Default);
        if let Some(message) = self.yaml.get_footer_message(forward) {
            self.footer.show_info(message, DEFAULT_MESSAGE_DURATION);
        }
    }

    fn process_event_internal(&mut self, event: &TuiEvent) -> ResponseEvent {
        if self.command_palette.is_visible {
            let result = self.process_command_palette_event(event);
            if result != ResponseEvent::NotHandled || event.is_mouse(MouseEventKind::LeftClick) {
                return result;
            }
        }

        if self.search.is_visible {
            let result = self.search.process_event(event);
            if self.yaml.search(self.search.value(), false) {
                self.yaml.scroll_to_current_match(None);
                self.update_search_count();
            }

            return result;
        }

        if self.modal.is_visible {
            return self.process_modal_event(event);
        }

        if self.app_data.has_binding(event, KeyCommand::YamlEdit) && self.enable_edit_mode() {
            return ResponseEvent::Handled;
        }

        if self.app_data.has_binding(event, KeyCommand::NavigateBack) {
            if self.is_new {
                return self.process_view_close_event(ResponseEvent::Cancelled, false);
            } else if self.is_edit && !self.yaml.is_modified() {
                return ResponseEvent::Cancelled;
            } else if self.yaml.disable_edit_mode() {
                self.hide_edit_hint();
                return ResponseEvent::Handled;
            }
        }

        if let TuiEvent::Mouse(mouse) = event
            && mouse.kind == MouseEventKind::RightClick
        {
            self.show_mouse_menu(mouse.column, mouse.row);
            return ResponseEvent::Handled;
        }

        if self.yaml.is_in_edit_mode() {
            if event.is_key(&KeyCombination::new(KeyCode::Char('v'), KeyModifiers::CONTROL)) {
                self.insert_from_clipboard();
                self.yaml.scroll_to_cursor();
                return ResponseEvent::Handled;
            }

            let is_ctrl_c = event.is_key(&KeyCombination::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
            if is_ctrl_c || event.is_key(&KeyCombination::new(KeyCode::Char('x'), KeyModifiers::CONTROL)) {
                self.copy_to_clipboard(true);
                if is_ctrl_c {
                    return ResponseEvent::Handled;
                }
            }
        }

        let response = self.yaml.process_event(event);
        if response != ResponseEvent::NotHandled {
            return response;
        }

        if self.yaml.is_in_edit_mode() {
            return ResponseEvent::NotHandled;
        }

        if self.app_data.has_binding(event, KeyCommand::CommandPaletteOpen) {
            self.show_command_palette();
            return ResponseEvent::Handled;
        }

        if self.app_data.has_binding(event, KeyCommand::SearchOpen) {
            self.search.show();
            return ResponseEvent::Handled;
        }

        if self.app_data.has_binding(event, KeyCommand::SearchReset) && !self.search.value().is_empty() {
            self.clear_search();
            return ResponseEvent::Handled;
        }

        if self.app_data.has_binding(event, KeyCommand::NavigateBack) {
            return self.process_view_close_event(ResponseEvent::Cancelled, false);
        }

        if self.app_data.has_binding(event, KeyCommand::YamlDecode) && self.yaml.header.kind.as_str() == SECRETS {
            self.toggle_yaml_decode();
            return ResponseEvent::Handled;
        }

        if self.app_data.has_binding(event, KeyCommand::ContentCopy) {
            self.copy_to_clipboard(false);
            return ResponseEvent::Handled;
        }

        if self.app_data.has_binding(event, KeyCommand::MatchNext) && self.yaml.matches_count().is_some() {
            self.navigate_match(true);
        }

        if self.app_data.has_binding(event, KeyCommand::MatchPrevious) && self.yaml.matches_count().is_some() {
            self.navigate_match(false);
        }

        ResponseEvent::NotHandled
    }

    fn process_command_palette_event(&mut self, event: &TuiEvent) -> ResponseEvent {
        let response = self.command_palette.process_event(event);

        if response == ResponseEvent::Cancelled {
            self.clear_search();
        } else if response.is_action("back") {
            self.last_mouse_click = event.position();
            return self.process_event(&TuiEvent::Command(KeyCommand::NavigateBack));
        } else if response.is_action("palette") {
            self.last_mouse_click = event.position();
            return self.process_event(&TuiEvent::Command(KeyCommand::CommandPaletteOpen));
        } else if response.is_action("copy") {
            self.copy_to_clipboard(false);
            return ResponseEvent::Handled;
        } else if response.is_action("copy_2") {
            return self.process_event(&KeyCombination::new(KeyCode::Char('c'), KeyModifiers::CONTROL).into());
        } else if response.is_action("paste") {
            return self.process_event(&KeyCombination::new(KeyCode::Char('v'), KeyModifiers::CONTROL).into());
        } else if response.is_action("cut") {
            return self.process_event(&KeyCombination::new(KeyCode::Char('x'), KeyModifiers::CONTROL).into());
        } else if response.is_action("undo") {
            return self.process_event(&KeyCombination::new(KeyCode::Char('z'), KeyModifiers::CONTROL).into());
        } else if response.is_action("redo") {
            return self.process_event(&KeyCombination::new(KeyCode::Char('y'), KeyModifiers::CONTROL).into());
        } else if response.is_action("decode") {
            self.toggle_yaml_decode();
            return ResponseEvent::Handled;
        } else if response.is_action("search") {
            self.search.highlight_position(event.position());
            self.search.show();
            return ResponseEvent::Handled;
        } else if response.is_action("edit") && self.enable_edit_mode() {
            return ResponseEvent::Handled;
        }

        if (response == ResponseEvent::Cancelled || response == ResponseEvent::ExitApplication) && self.yaml.is_modified() {
            let is_quit = response == ResponseEvent::ExitApplication;
            self.last_mouse_click = event.position();
            return self.process_view_close_event(response, is_quit);
        }

        response
    }

    fn process_modal_event(&mut self, event: &TuiEvent) -> ResponseEvent {
        let force = self.modal.checkbox(0).is_some_and(|i| i.is_checked);
        let patch_status = self.modal.checkbox(1).is_some_and(|i| i.is_checked);
        let disable_encoding = self.modal.checkbox(2).is_some_and(|i| i.is_checked);
        let response = self.modal.process_event(event);
        if response.is_action("apply") {
            return self.save_yaml(true, force, disable_encoding, patch_status);
        } else if response.is_action("patch") {
            return self.save_yaml(false, false, disable_encoding, patch_status);
        } else if response.is_action("create") {
            return self.create_resource(disable_encoding, patch_status);
        }

        response
    }

    fn process_view_close_event(&mut self, response: ResponseEvent, is_quit: bool) -> ResponseEvent {
        if self.yaml.is_modified() {
            self.modal = self.new_save_dialog(response);
            self.modal.show();
            self.state = if is_quit {
                ViewState::WaitingForQuit
            } else {
                ViewState::WaitingForClose
            };
            ResponseEvent::Handled
        } else {
            response
        }
    }

    fn new_save_dialog(&mut self, response: ResponseEvent) -> Dialog {
        if self.is_new {
            self.new_save_new_dialog(response)
        } else {
            self.new_save_existing_dialog(response)
        }
    }

    fn new_save_new_dialog(&mut self, response: ResponseEvent) -> Dialog {
        let colors = &self.app_data.borrow().theme.colors.modal;
        let mut inputs = Vec::new();
        if let Some(content) = self.yaml.content() {
            let kind = deserialize_kind(&content.plain);
            if self.can_patch_status
                && let Some(origin) = self.origin_kind.as_deref()
                && kind.as_deref().is_some_and(|k| k == origin)
            {
                inputs.push(CheckBox::new(1, "Create along with status", false, &colors.checkbox));
            }

            if kind.as_deref().is_some_and(|k| k == "Secret") {
                inputs.push(CheckBox::new(2, "Do not encode data fields", false, &colors.checkbox));
            }
        }

        Dialog::new(
            "You have made changes to the new resource's YAML. Do you want to create it now?".to_owned(),
            vec![
                Button::new("Create", ResponseEvent::Action("create"), &colors.btn_accent),
                Button::new("Discard", response, &colors.btn_delete),
                Button::new("Cancel", ResponseEvent::Action("cancel"), &colors.btn_cancel),
            ],
            60,
            colors.text,
        )
        .with_checkboxes(inputs)
        .with_highlighted_position(self.last_mouse_click.take())
    }

    fn new_save_existing_dialog(&mut self, response: ResponseEvent) -> Dialog {
        let colors = &self.app_data.borrow().theme.colors.modal;
        let mut inputs = vec![CheckBox::new(0, "Force ownership (apply only)", false, &colors.checkbox)];
        if self.can_patch_status {
            inputs.push(CheckBox::new(1, "Update along with status", false, &colors.checkbox));
        }
        if self.is_secret {
            inputs.push(CheckBox::new(2, "Do not encode data fields", false, &colors.checkbox));
        }

        Dialog::new(
            "You have made changes to the resource's YAML. Do you want to apply / patch them now?".to_owned(),
            vec![
                Button::new("Apply", ResponseEvent::Action("apply"), &colors.btn_accent),
                Button::new("Patch", ResponseEvent::Action("patch"), &colors.btn_accent),
                Button::new("Discard", response, &colors.btn_delete),
                Button::new("Cancel", ResponseEvent::Action("cancel"), &colors.btn_cancel),
            ],
            60,
            colors.text,
        )
        .with_checkboxes(inputs)
        .with_highlighted_position(self.last_mouse_click.take())
    }

    fn create_resource(&mut self, disable_encoding: bool, patch_status: bool) -> ResponseEvent {
        if let Some(yaml) = self.yaml.content() {
            let kind = deserialize_kind(&yaml.plain);
            let encode = kind.as_deref().is_some_and(|k| k == "Secret") && !disable_encoding;
            let options = SetNewResourceYamlOptions { encode, patch_status };
            let yaml = yaml.plain.join("\n");

            self.command_id = self.worker.borrow_mut().set_new_yaml(yaml, options);

            ResponseEvent::Handled
        } else {
            ResponseEvent::Cancelled
        }
    }

    fn save_yaml(&mut self, is_apply: bool, is_forced: bool, disable_encoding: bool, patch_status: bool) -> ResponseEvent {
        if let Some(yaml) = self.yaml.content() {
            let name = self.yaml.header.name.as_deref().map(String::from).unwrap_or_default();
            let namespace = self.yaml.header.namespace.clone();
            let kind = &self.yaml.header.kind;
            let yaml = yaml.plain.join("\n");
            let encode = self.is_secret && !disable_encoding;
            let action = match (is_apply, is_forced) {
                (true, true) => SetResourceYamlAction::ForceApply,
                (true, false) => SetResourceYamlAction::Apply,
                _ => SetResourceYamlAction::Patch,
            };
            let options = SetResourceYamlOptions {
                action,
                encode,
                patch_status,
            };

            self.command_id = self.worker.borrow_mut().set_yaml(name, namespace, kind, yaml, options);

            ResponseEvent::Handled
        } else {
            ResponseEvent::Cancelled
        }
    }

    fn enable_edit_mode(&mut self) -> bool {
        if self.is_secret && !self.is_decoded {
            self.toggle_yaml_decode();
            self.state = ViewState::WaitingForEdit;
            return false;
        }

        if self.yaml.enable_edit_mode(self.is_new) {
            self.clear_search();
            self.show_edit_hint();
            return true;
        }

        false
    }

    fn process_new_content(&mut self, result: ResourceYamlResult) {
        let Some(highlighter) = self.worker.borrow().get_highlighter() else {
            return;
        };
        let name = if self.is_new { None } else { Some(result.name) };
        let icon = if result.is_decoded { '' } else { '' };
        let styles = {
            let colors = &self.app_data.borrow().theme.colors.syntax.yaml;
            StyleFallback {
                excluded: (&colors.normal).into(),
                fallback: (&colors.string).into(),
            }
        };
        self.is_decoded = result.is_decoded;
        self.can_patch_status = result.can_patch_status;
        self.origin_kind = Some(result.singular);
        self.yaml.header.set_icon(icon);
        self.yaml.header.set_data(result.namespace, result.kind, name, None);
        self.yaml.set_content(YamlContent::new(
            result.styled,
            result.yaml,
            highlighter,
            result.is_editable,
            styles,
        ));
        if self.is_new || self.state == ViewState::WaitingForEdit {
            self.state = ViewState::Idle;
            if self.yaml.enable_edit_mode(self.is_new) {
                self.show_edit_hint();
            }
        }
    }

    fn update_view_state(&mut self) {
        if self.state == ViewState::WaitingForClose {
            self.state = ViewState::Closing;
        } else if self.state == ViewState::WaitingForQuit {
            self.state = ViewState::Quitting;
        } else {
            self.state = ViewState::Idle;
        }
    }

    fn show_edit_hint(&mut self) {
        self.is_hint_visible = true;
        let key = self.app_data.get_key_name(KeyCommand::NavigateBack).to_ascii_uppercase();
        if self.is_new {
            self.footer
                .show_hint(format!(" Press {key} to open save dialog (if modified)"));
        } else {
            self.footer.show_hint(format!(
                " Press {key} to close edit mode, then {key} for save dialog (if modified)"
            ));
        }
    }

    fn hide_edit_hint(&mut self) {
        if self.is_hint_visible {
            self.is_hint_visible = true;
            self.footer.hide_hint();
        }
    }
}

impl View for YamlView {
    fn command_id(&self) -> Option<&str> {
        self.command_id.as_deref()
    }

    fn process_command_result(&mut self, result: CommandResult) {
        match result {
            CommandResult::GetNewResourceYaml(Ok(result)) => {
                self.process_new_content(result.into());
            },
            CommandResult::GetResourceYaml(Ok(result)) => {
                self.process_new_content(result);
            },
            CommandResult::SetNewResourceYaml(Ok(name)) => {
                self.update_view_state();
                self.footer.show_info(format!("'{name}' created successfully"), 3_000);
            },
            CommandResult::SetResourceYaml(Ok(name)) => {
                self.update_view_state();
                self.footer.show_info(format!("'{name}' YAML saved successfully"), 3_000);
            },
            _ => (),
        }
    }

    fn process_tick(&mut self) -> ResponseEvent {
        if self.state == ViewState::Quitting {
            return ResponseEvent::ExitApplication;
        } else if self.state == ViewState::Closing {
            return ResponseEvent::Cancelled;
        }

        self.yaml.process_tick()
    }

    fn process_disconnection(&mut self) {
        self.command_palette.hide();
    }

    fn process_event(&mut self, event: &TuiEvent) -> ResponseEvent {
        let result = self.process_event_internal(event);

        if result == ResponseEvent::Cancelled || result == ResponseEvent::ExitApplication {
            self.hide_edit_hint();
        }

        result
    }

    fn draw(&mut self, frame: &mut Frame<'_>, area: Rect) {
        self.yaml.draw(frame, area, None);
        self.command_palette.draw(frame, frame.area());
        self.search.draw(frame, frame.area());
        self.modal.draw(frame, area);
    }
}

#[derive(PartialEq)]
enum ViewState {
    Idle,
    WaitingForEdit,
    WaitingForClose,
    WaitingForQuit,
    Closing,
    Quitting,
}
