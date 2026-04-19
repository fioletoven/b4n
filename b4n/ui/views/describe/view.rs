use b4n_common::NotificationSink;
use b4n_config::keys::KeyCommand;
use b4n_kube::{ResourceRef, client::KubernetesClient};
use b4n_tui::{ResponseEvent, Responsive, TuiEvent, widgets::ActionsListBuilder};
use kube::Client;
use ratatui::{Frame, layout::Rect};
use std::rc::Rc;
use tokio::runtime::Handle;

use crate::core::{SharedAppData, SharedAppDataExt};
use crate::ui::{views::View, widgets::CommandPalette};

/// Pod's describe view.
pub struct DescribeView {
    app_data: SharedAppData,
    client: Client,
    resource: ResourceRef,
    command_palette: CommandPalette,
    footer_tx: NotificationSink,
}

impl DescribeView {
    /// Creates new [`DescribeView`] instance.
    pub fn new(
        runtime: Handle,
        app_data: SharedAppData,
        client: &KubernetesClient,
        resource: ResourceRef,
        footer_tx: NotificationSink,
    ) -> Self {
        Self {
            app_data,
            client: client.get_client(),
            resource,
            command_palette: CommandPalette::default(),
            footer_tx,
        }
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
