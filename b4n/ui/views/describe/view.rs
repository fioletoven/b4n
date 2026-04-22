use b4n_common::NotificationSink;
use b4n_config::keys::KeyCommand;
use b4n_kube::utils::get_resource;
use b4n_kube::{EVENTS, ResourceRefFilter};
use b4n_kube::{Kind, ResourceRef};
use b4n_tui::{ResponseEvent, Responsive, TuiEvent, widgets::ActionsListBuilder};
use kube::Client;
use ratatui::{Frame, layout::Rect};
use std::rc::Rc;

use crate::core::{SharedAppData, SharedAppDataExt, SharedBgWorker};
use crate::kube::resources::ResourceObserver;
use crate::ui::{views::View, widgets::CommandPalette};

/// Pod's describe view.
pub struct DescribeView {
    app_data: SharedAppData,
    client: Client,
    resource: ResourceRef,
    events: ResourceObserver,
    command_palette: CommandPalette,
    footer_tx: NotificationSink,
}

impl DescribeView {
    /// Creates new [`DescribeView`] instance.
    pub fn new(
        worker: SharedBgWorker,
        app_data: SharedAppData,
        resource: ResourceRef,
        uid: &str,
        footer_tx: NotificationSink,
    ) -> Option<Self> {
        let worker = worker.borrow();
        let resource_name = resource.name.as_deref().map(String::from)?;
        let client = worker.kubernetes_client()?;

        let runtime = worker.runtime_handle().clone();

        let events_filter = ResourceRefFilter::involved(resource_name, uid);
        let events_kind = Kind::from(EVENTS);
        let events_dis = get_resource(worker.discovery_list(), &events_kind);
        let events_res = ResourceRef::filtered(events_kind, resource.namespace.clone(), events_filter);
        let mut events = ResourceObserver::simple(runtime);
        events.start(client, events_res, events_dis, true).ok()?;

        Some(Self {
            app_data,
            client: client.get_client(),
            resource,
            events,
            command_palette: CommandPalette::default(),
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

        self.command_palette = CommandPalette::new(Rc::clone(&self.app_data), actions, 65);
        self.command_palette.show();
        self.footer_tx.hide_hint();
    }
}

impl View for DescribeView {
    fn process_disconnection(&mut self) {
        self.command_palette.hide();
    }

    fn process_event(&mut self, event: &TuiEvent) -> ResponseEvent {
        if self.command_palette.is_visible {
            return self.command_palette.process_event(event);
        }

        if self.app_data.has_binding(event, KeyCommand::NavigateBack) {
            return ResponseEvent::Cancelled;
        }

        if self.app_data.has_binding(event, KeyCommand::CommandPaletteOpen) {
            self.show_command_palette();
            return ResponseEvent::Handled;
        }

        ResponseEvent::NotHandled
    }

    fn draw(&mut self, frame: &mut Frame<'_>, area: Rect) {
        self.command_palette.draw(frame, area);
    }
}
