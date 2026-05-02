use b4n_common::NotificationSink;
use b4n_config::keys::KeyCommand;
use b4n_kube::utils::get_resource;
use b4n_kube::{BgObserver, EVENTS, ResourceRefFilter};
use b4n_kube::{Kind, ResourceRef};
use b4n_tui::MouseEventKind;
use b4n_tui::widgets::ActionItem;
use b4n_tui::{ResponseEvent, Responsive, TuiEvent, widgets::ActionsListBuilder};
use ratatui::layout::{Constraint, Direction, Layout, Position};
use ratatui::{Frame, layout::Rect};
use std::rc::Rc;

use crate::core::{SharedAppData, SharedAppDataExt, SharedBgWorker};
use crate::kube::resources::{ColumnsLayout, ResourceObserver};
use crate::ui::presentation::ContentHeader;
use crate::ui::views::describe::content::DescribeContent;
use crate::ui::{views::View, widgets::CommandPalette};

/// Pod's describe view.
pub struct DescribeView {
    app_data: SharedAppData,
    header: ContentHeader,
    content: DescribeContent,
    observer: BgObserver,
    events: ResourceObserver,
    command_palette: CommandPalette,
    last_mouse_click: Option<Position>,
    footer_tx: NotificationSink,
}

impl DescribeView {
    /// Creates new [`DescribeView`] instance.
    pub fn new(
        worker: &SharedBgWorker,
        app_data: SharedAppData,
        resource: ResourceRef,
        uid: &str,
        footer_tx: NotificationSink,
    ) -> Option<Self> {
        let worker = worker.borrow();
        let resource_name = resource.name.as_deref().map(String::from)?;
        let client = worker.kubernetes_client()?;

        let runtime = worker.runtime_handle().clone();
        let discovery = get_resource(worker.discovery_list(), &resource.kind);
        let mut observer = BgObserver::new(runtime, None);
        observer
            .start(client.get_client(), resource.clone(), discovery, None, false)
            .ok()?;

        let runtime = worker.runtime_handle().clone();
        let events_filter = ResourceRefFilter::involved(resource_name, uid);
        let events_kind = Kind::from(EVENTS);
        let events_dis = get_resource(worker.discovery_list(), &events_kind);
        let events_res = ResourceRef::filtered(events_kind, resource.namespace.clone(), events_filter);
        let mut events = ResourceObserver::simple(runtime).with_columns_layout(ColumnsLayout::Compact);
        events.start(client, events_res, events_dis, true).ok()?;

        let mut header = ContentHeader::new(Rc::clone(&app_data), true);
        header.set_title(" describe");
        header.set_data(resource.namespace.clone(), resource.kind.clone(), resource.name.clone(), None);
        let content = DescribeContent::new(Rc::clone(&app_data), resource);

        set_hint(&app_data, &footer_tx);

        Some(Self {
            app_data,
            header,
            content,
            observer,
            events,
            command_palette: CommandPalette::default(),
            last_mouse_click: None,
            footer_tx,
        })
    }

    /// Shows command palette.
    fn show_command_palette(&mut self) {
        let builder = ActionsListBuilder::default()
            .with_back()
            .with_quit()
            .with_aliases(&self.app_data.borrow().config.aliases);
        let actions = builder.build(Some(&self.app_data.borrow().key_bindings));

        self.command_palette =
            CommandPalette::new(Rc::clone(&self.app_data), actions, 65).with_highlighted_position(self.last_mouse_click.take());
        self.command_palette.show();
        self.footer_tx.hide_hint();
    }

    /// Shows menu for right mouse button.
    fn show_mouse_menu(&mut self, x: u16, y: u16) {
        if !self.app_data.borrow().is_connected() {
            return;
        }

        let builder = ActionsListBuilder::default()
            .with_menu_action(ActionItem::back())
            .with_menu_action(ActionItem::command_palette());

        self.command_palette = CommandPalette::new(Rc::clone(&self.app_data), builder.build(None), 22).to_mouse_menu();
        self.command_palette.show_at((x.saturating_sub(3), y).into());
    }

    /// Processes events that are from the command palette.
    fn process_command_palette_event(&mut self, event: &TuiEvent) -> ResponseEvent {
        match self.command_palette.process_event(event) {
            ResponseEvent::Action("palette") => {
                self.last_mouse_click = event.position();
                self.process_event(&TuiEvent::Command(KeyCommand::CommandPaletteOpen))
            },
            response_event => response_event,
        }
    }
}

impl View for DescribeView {
    fn process_tick(&mut self) -> ResponseEvent {
        while let Some(result) = self.observer.try_next() {
            self.content.update_resource(*result);
        }

        while let Some(result) = self.events.try_next() {
            self.content.update_events(*result);
        }

        ResponseEvent::Handled
    }

    fn process_disconnection(&mut self) {
        self.command_palette.hide();
    }

    fn process_event(&mut self, event: &TuiEvent) -> ResponseEvent {
        if self.command_palette.is_visible {
            let result = self.process_command_palette_event(event);
            if result != ResponseEvent::NotHandled {
                return result;
            }
        }

        if self.app_data.has_binding(event, KeyCommand::NavigateBack) {
            return ResponseEvent::Cancelled;
        }

        if self.app_data.has_binding(event, KeyCommand::CommandPaletteOpen) {
            self.show_command_palette();
            return ResponseEvent::Handled;
        }

        if let TuiEvent::Mouse(mouse) = event
            && mouse.kind == MouseEventKind::RightClick
        {
            self.show_mouse_menu(mouse.column, mouse.row);
            return ResponseEvent::Handled;
        }

        self.content.process_event(event)
    }

    fn draw(&mut self, frame: &mut Frame<'_>, area: Rect) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(1), Constraint::Fill(1)])
            .split(area);

        if let Some(pos) = self.content.get_coordinates() {
            self.header.set_coordinates(pos.x, pos.y);
        } else {
            self.header.hide_coordinates();
        }

        self.header.draw(frame, layout[0]);
        self.content.draw(frame, layout[1]);

        self.command_palette.draw(frame, area);
    }
}

impl Drop for DescribeView {
    fn drop(&mut self) {
        self.footer_tx.hide_hint();
    }
}

fn set_hint(app_data: &SharedAppData, footer_tx: &NotificationSink) {
    let key = app_data.get_key_name(KeyCommand::NavigateNext).to_ascii_uppercase();
    footer_tx.show_hint(format!(" Press ␝{key}␝ to change active section"));
}
