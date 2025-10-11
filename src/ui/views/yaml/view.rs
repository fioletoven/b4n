use ratatui::{Frame, layout::Rect};
use std::rc::Rc;

use crate::{
    core::{
        SharedAppData, SharedAppDataExt, SharedBgWorker,
        commands::{CommandResult, SetResourceYamlAction},
    },
    kubernetes::{ResourceRef, resources::SECRETS},
    ui::{
        KeyCommand, MouseEventKind, ResponseEvent, Responsive, TuiEvent,
        viewers::ContentViewer,
        views::{View, yaml::YamlContent},
        widgets::{ActionItem, ActionsListBuilder, Button, CheckBox, CommandPalette, Dialog, FooterTx, IconKind, Search},
    },
};

/// YAML view.
pub struct YamlView {
    yaml: ContentViewer<YamlContent>,
    app_data: SharedAppData,
    worker: SharedBgWorker,
    is_decoded: bool,
    command_id: Option<String>,
    command_palette: CommandPalette,
    search: Search,
    modal: Dialog,
    footer: FooterTx,
}

impl YamlView {
    /// Creates new [`YamlView`] instance.
    pub fn new(
        app_data: SharedAppData,
        worker: SharedBgWorker,
        command_id: Option<String>,
        resource: ResourceRef,
        footer: FooterTx,
    ) -> Self {
        let color = app_data.borrow().theme.colors.syntax.yaml.search;
        let yaml = ContentViewer::new(Rc::clone(&app_data), color).with_header(
            "YAML",
            '',
            resource.namespace,
            resource.kind,
            resource.name.unwrap_or_default(),
            None,
        );
        let search = Search::new(Rc::clone(&app_data), Some(Rc::clone(&worker)), 60);

        Self {
            yaml,
            app_data,
            worker,
            is_decoded: false,
            command_id,
            command_palette: CommandPalette::default(),
            search,
            modal: Dialog::default(),
            footer,
        }
    }

    fn copy_yaml_to_clipboard(&self) {
        if self.yaml.content().is_some() {
            if let Some(clipboard) = &mut self.app_data.borrow_mut().clipboard
                && clipboard
                    .set_text(self.yaml.content().map(|c| c.plain.join("\n")).unwrap_or_default())
                    .is_ok()
            {
                self.footer.show_info(" YAML content copied to clipboard…", 1_500);
            } else {
                self.footer.show_error(" Unable to access clipboard functionality…", 2_000);
            }
        }
    }

    fn show_command_palette(&mut self) {
        let mut builder = ActionsListBuilder::default()
            .with_close()
            .with_quit()
            .with_action(
                ActionItem::new("copy")
                    .with_description("copies YAML to the clipboard")
                    .with_response(ResponseEvent::Action("copy")),
            )
            .with_action(
                ActionItem::new("search")
                    .with_description("searches YAML using the provided query")
                    .with_response(ResponseEvent::Action("search")),
            )
            .with_action(
                ActionItem::new("edit")
                    .with_description("switches to the edit mode")
                    .with_aliases(&["insert"])
                    .with_response(ResponseEvent::Action("edit")),
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
    }

    fn toggle_yaml_decode(&mut self) {
        self.command_id = self.worker.borrow_mut().get_yaml(
            self.yaml.header.name.clone(),
            self.yaml.header.namespace.clone(),
            &self.yaml.header.kind,
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
            self.footer.show_info(message, 0);
        }
    }

    fn new_save_dialog(&mut self, response: ResponseEvent) -> Dialog {
        let colors = &self.app_data.borrow().theme.colors;

        Dialog::new(
            "You have made changes to the resource's YAML. Do you want to apply / patch them now?".to_owned(),
            vec![
                Button::new("Apply", ResponseEvent::Action("apply"), &colors.modal.btn_accent),
                Button::new("Patch", ResponseEvent::Action("patch"), &colors.modal.btn_accent),
                Button::new("Leave", response, &colors.modal.btn_delete),
                Button::new("Cancel", ResponseEvent::Action("cancel"), &colors.modal.btn_cancel),
            ],
            60,
            colors.modal.text,
        )
        .with_inputs(vec![CheckBox::new(
            "Force ownership (apply only)",
            false,
            &colors.modal.checkbox,
        )])
    }

    fn process_command_palette_event(&mut self, event: &TuiEvent) -> ResponseEvent {
        let response = self.command_palette.process_event(event);

        if response == ResponseEvent::Cancelled {
            self.clear_search();
        } else if response.is_action("copy") {
            self.copy_yaml_to_clipboard();
            return ResponseEvent::Handled;
        } else if response.is_action("decode") {
            self.toggle_yaml_decode();
            return ResponseEvent::Handled;
        } else if response.is_action("search") {
            self.search.show();
            return ResponseEvent::Handled;
        } else if response.is_action("edit") && self.yaml.enable_edit_mode() {
            return ResponseEvent::Handled;
        }

        if (response == ResponseEvent::Cancelled || response == ResponseEvent::ExitApplication) && self.yaml.is_modified() {
            return self.process_view_close_event(response);
        }

        response
    }

    fn process_modal_event(&mut self, event: &TuiEvent) -> ResponseEvent {
        let response = self.modal.process_event(event);
        if response.is_action("apply") {
            let force = self.modal.input(0).map(|i| i.is_checked).unwrap_or_default();
            return self.save_yaml(true, force);
        } else if response.is_action("patch") {
            return self.save_yaml(false, false);
        }

        response
    }

    fn process_view_close_event(&mut self, response: ResponseEvent) -> ResponseEvent {
        if self.yaml.is_modified() {
            self.modal = self.new_save_dialog(response);
            self.modal.show();
            ResponseEvent::Handled
        } else {
            response
        }
    }

    fn save_yaml(&mut self, is_apply: bool, is_forced: bool) -> ResponseEvent {
        if let Some(yaml) = self.yaml.content() {
            let name = self.yaml.header.name.clone();
            let namespace = self.yaml.header.namespace.clone();
            let kind = &self.yaml.header.kind;
            let yaml = yaml.plain.join("\n");
            let action = match (is_apply, is_forced) {
                (true, true) => SetResourceYamlAction::ForceApply,
                (true, false) => SetResourceYamlAction::Apply,
                _ => SetResourceYamlAction::Patch,
            };

            self.command_id = self.worker.borrow_mut().set_yaml(name, namespace, kind, yaml, action);

            ResponseEvent::Handled
        } else {
            ResponseEvent::Cancelled
        }
    }
}

impl View for YamlView {
    fn command_id(&self) -> Option<&str> {
        self.command_id.as_deref()
    }

    fn process_tick(&mut self) -> ResponseEvent {
        self.yaml.process_tick()
    }

    fn process_command_result(&mut self, result: CommandResult) {
        if let CommandResult::GetResourceYaml(Ok(result)) = result
            && let Some(highlighter) = self.worker.borrow().get_higlighter()
        {
            let icon = if result.is_decoded { '' } else { '' };
            self.is_decoded = result.is_decoded;
            self.yaml.header.set_icon(icon);
            self.yaml.header.set_data(result.namespace, result.kind, result.name, None);
            self.yaml
                .set_content(YamlContent::new(result.styled, result.yaml, highlighter, result.is_editable));
        }
    }

    fn process_disconnection(&mut self) {
        self.command_palette.hide();
    }

    fn process_event(&mut self, event: &TuiEvent) -> ResponseEvent {
        if self.command_palette.is_visible {
            return self.process_command_palette_event(event);
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

        if self.app_data.has_binding(event, KeyCommand::YamlEdit) && self.yaml.enable_edit_mode() {
            return ResponseEvent::Handled;
        }

        if self.app_data.has_binding(event, KeyCommand::NavigateBack) && self.yaml.disable_edit_mode() {
            return ResponseEvent::Handled;
        }

        let response = self.yaml.process_event(event);
        if response != ResponseEvent::NotHandled {
            return response;
        }

        if self.app_data.has_binding(event, KeyCommand::CommandPaletteOpen) || event.is(MouseEventKind::RightClick) {
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
            return self.process_view_close_event(ResponseEvent::Cancelled);
        }

        if self.app_data.has_binding(event, KeyCommand::YamlDecode)
            && self.yaml.header.kind.as_str() == SECRETS
            && self.app_data.borrow().is_connected
        {
            self.toggle_yaml_decode();
            self.clear_search();
            return ResponseEvent::Handled;
        }

        if self.app_data.has_binding(event, KeyCommand::ContentCopy) {
            self.copy_yaml_to_clipboard();
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

    fn draw(&mut self, frame: &mut Frame<'_>, area: Rect) {
        self.yaml.draw(frame, area, None);
        self.command_palette.draw(frame, frame.area());
        self.search.draw(frame, frame.area());
        self.modal.draw(frame, area);
    }
}
