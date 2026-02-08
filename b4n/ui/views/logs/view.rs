use b4n_common::{DEFAULT_MESSAGE_DURATION, IconKind, NotificationSink};
use b4n_config::keys::KeyCommand;
use b4n_kube::client::KubernetesClient;
use b4n_kube::{PODS, PodRef};
use b4n_tui::widgets::{ActionItem, ActionsListBuilder};
use b4n_tui::{MouseEventKind, ResponseEvent, Responsive, TuiEvent};
use crossterm::event::KeyCode;
use ratatui::Frame;
use ratatui::layout::{Position, Rect};
use std::rc::Rc;

use crate::core::{SharedAppData, SharedAppDataExt, SharedBgWorker};
use crate::ui::presentation::{Content, ContentViewer};
use crate::ui::views::View;
use crate::ui::views::logs::content::{LogsContent, TIMESTAMP_TEXT_LENGTH};
use crate::ui::views::logs::{LogsObserver, LogsObserverError};
use crate::ui::widgets::{CommandPalette, Search};

/// Possible errors from [`LogsObserver`].
#[derive(thiserror::Error, Debug)]
pub enum LogsViewError {
    /// No containers to observe provided.
    #[error("no containers provided")]
    NoContainersToObserve,

    /// Kubernetes client error.
    #[error("kubernetes client error")]
    ObserverError(#[from] LogsObserverError),
}

/// Logs view.
pub struct LogsView {
    logs: ContentViewer<LogsContent>,
    app_data: SharedAppData,
    observer: LogsObserver,
    last_mouse_click: Option<Position>,
    command_palette: CommandPalette,
    search: Search,
    footer: NotificationSink,
    bound_to_bottom: bool,
    area: Rect,
}

impl LogsView {
    /// Creates new [`LogsView`] instance.
    pub fn new(
        app_data: SharedAppData,
        worker: SharedBgWorker,
        client: &KubernetesClient,
        mut containers: Vec<PodRef>,
        previous: bool,
        footer: NotificationSink,
        workspace: Rect,
    ) -> Result<Self, LogsViewError> {
        if containers.is_empty() {
            return Err(LogsViewError::NoContainersToObserve);
        }

        let select = app_data.borrow().theme.colors.syntax.logs.select;
        let search = app_data.borrow().theme.colors.syntax.logs.search;
        let area = ContentViewer::<LogsContent>::get_content_area(workspace);
        let container = (containers.len() == 1).then(|| containers[0].container.clone()).flatten();
        let logs = ContentViewer::new(Rc::clone(&app_data), select, search, area).with_header(
            if previous { "previous logs" } else { "logs" },
            '',
            containers[0].namespace.clone(),
            PODS.into(),
            Some(containers[0].name.clone()),
            container,
        );

        let pod = containers.swap_remove(0);
        let mut observer = LogsObserver::new(worker.borrow().runtime_handle().clone());
        observer.start(client, pod, app_data.borrow().config.logs.lines, previous, false);
        let search = Search::new(Rc::clone(&app_data), Some(worker), 65);

        Ok(Self {
            logs,
            app_data,
            observer,
            last_mouse_click: None,
            command_palette: CommandPalette::default(),
            search,
            footer,
            bound_to_bottom: true,
            area: workspace,
        })
    }

    fn show_command_palette(&mut self) {
        let builder = ActionsListBuilder::default()
            .with_back()
            .with_quit()
            .with_action(
                ActionItem::action("timestamps", "timestamps").with_description("toggles the display of timestamps"),
                Some(KeyCommand::LogsTimestamps),
            )
            .with_action(
                ActionItem::action("copy", "copy").with_description("copies logs to clipboard"),
                Some(KeyCommand::ContentCopy),
            )
            .with_action(
                ActionItem::action("search", "search").with_description("searches logs using the provided query"),
                Some(KeyCommand::SearchOpen),
            );
        let actions = builder.build(self.app_data.borrow().config.key_bindings.as_ref());
        self.command_palette =
            CommandPalette::new(Rc::clone(&self.app_data), actions, 65).with_highlighted_position(self.last_mouse_click.take());
        self.command_palette.show();
        self.footer.hide_hint();
    }

    fn show_mouse_menu(&mut self, x: u16, y: u16) {
        let copy = if self.logs.has_selection() { "selection" } else { "all" };
        let builder = ActionsListBuilder::default()
            .with_menu_action(ActionItem::back())
            .with_menu_action(ActionItem::command_palette())
            .with_menu_action(ActionItem::menu(1, &format!("󰆏 copy ␝{copy}␝"), "copy"))
            .with_menu_action(ActionItem::menu(2, " search", "search"));
        self.command_palette = CommandPalette::new(Rc::clone(&self.app_data), builder.build(None), 22).to_mouse_menu();
        self.command_palette.show_at((x.saturating_sub(3), y).into());
    }

    fn toggle_timestamps(&mut self) {
        self.logs.clear_selection();
        if let Some(content) = self.logs.content_mut() {
            content.toggle_timestamps();
            self.logs.reset_horizontal_scroll();
        }
    }

    fn copy_logs_to_clipboard(&mut self) {
        if self.logs.content().is_some() {
            let range = self.logs.get_selection();
            let text = self.logs.content().map(|c| c.to_plain_text(range)).unwrap_or_default();
            self.app_data.copy_to_clipboard(text, &self.footer, || {
                if self.logs.has_selection() {
                    "Selection copied to clipboard"
                } else {
                    "Container logs copied to clipboard"
                }
            });
        }
    }

    fn update_bound_to_bottom(&mut self) {
        self.bound_to_bottom = self.search.value().is_empty() && self.logs.is_at_end();
        self.logs.header.set_icon(if self.bound_to_bottom { '' } else { '' });
    }

    fn clear_search(&mut self) {
        self.logs.search("", false);
        self.search.reset();
        self.update_search_count();
        self.update_bound_to_bottom();
    }

    fn update_search_count(&mut self) {
        self.footer
            .set_text("900_logs_search", self.logs.get_footer_text(), IconKind::Default);
        self.search.set_matches(self.logs.matches_count());
    }

    fn navigate_match(&mut self, forward: bool) {
        self.logs.navigate_match(forward, self.get_offset());
        self.footer
            .set_text("900_logs_search", self.logs.get_footer_text(), IconKind::Default);
        if let Some(message) = self.logs.get_footer_message(forward) {
            self.footer.show_info(message, DEFAULT_MESSAGE_DURATION);
        }
    }

    fn get_offset(&self) -> Option<Position> {
        if self.logs.content().is_some_and(|c| c.show_timestamps()) {
            Some(Position::new(TIMESTAMP_TEXT_LENGTH as u16, 0))
        } else {
            None
        }
    }

    fn process_command_palette_event(&mut self, event: &TuiEvent) -> ResponseEvent {
        let response = self.command_palette.process_event(event);
        if response == ResponseEvent::Cancelled {
            self.clear_search();
        } else if response.is_action("palette") {
            self.last_mouse_click = event.position();
            return self.process_event(&TuiEvent::Command(KeyCommand::CommandPaletteOpen));
        } else if response.is_action("timestamps") {
            self.toggle_timestamps();
            return ResponseEvent::Handled;
        } else if response.is_action("copy") {
            self.copy_logs_to_clipboard();
            return ResponseEvent::Handled;
        } else if response.is_action("search") {
            self.search.highlight_position(event.position());
            self.search.show();
            return ResponseEvent::Handled;
        }

        response
    }
}

impl View for LogsView {
    fn process_tick(&mut self) -> ResponseEvent {
        if !self.observer.is_empty() {
            if !self.logs.has_content() {
                let mut content = LogsContent::new(self.app_data.borrow().theme.colors.syntax.logs.clone());
                content.set_timestamps(self.app_data.borrow().config.logs.timestamps.is_none_or(|t| t));
                self.logs.set_content(content);
            }

            let content = self.logs.content_mut().unwrap();
            while let Some(chunk) = self.observer.try_next() {
                content.add_logs_chunk(*chunk);
            }

            if self.bound_to_bottom {
                self.logs.scroll_to_end();
            }

            if self.logs.search(self.search.value(), true) {
                self.update_search_count();
            }
        }

        ResponseEvent::Handled
    }

    fn process_disconnection(&mut self) {
        // pass
    }

    fn process_event(&mut self, event: &TuiEvent) -> ResponseEvent {
        if self.command_palette.is_visible {
            let result = self.process_command_palette_event(event);
            if result != ResponseEvent::NotHandled || event.is_mouse(MouseEventKind::LeftClick) {
                return result;
            }
        }

        if self.search.is_visible {
            let result = self.search.process_event(event);
            if self.logs.search(self.search.value(), false) {
                self.logs.scroll_to_current_match(self.get_offset());
                self.update_search_count();
            }

            self.update_bound_to_bottom();
            return result;
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

        if self.app_data.has_binding(event, KeyCommand::SearchOpen) {
            self.search.show();
            return ResponseEvent::Handled;
        }

        if self.app_data.has_binding(event, KeyCommand::SearchReset) && !self.search.value().is_empty() {
            self.clear_search();
            return ResponseEvent::Handled;
        }

        if self.app_data.has_binding(event, KeyCommand::NavigateBack) {
            return ResponseEvent::Cancelled;
        }

        if self.app_data.has_binding(event, KeyCommand::LogsTimestamps) {
            self.toggle_timestamps();
            return ResponseEvent::Handled;
        }

        if self.app_data.has_binding(event, KeyCommand::ContentCopy) {
            self.copy_logs_to_clipboard();
            return ResponseEvent::Handled;
        }

        if self.app_data.has_binding(event, KeyCommand::MatchNext) && self.logs.matches_count().is_some() {
            self.navigate_match(true);
            return ResponseEvent::Handled;
        }

        if self.app_data.has_binding(event, KeyCommand::MatchPrevious) && self.logs.matches_count().is_some() {
            self.navigate_match(false);
            return ResponseEvent::Handled;
        }

        if let TuiEvent::Key(key) = event
            && (key.code == KeyCode::Down || key.code == KeyCode::End || key.code == KeyCode::PageDown)
            && self.logs.is_at_end()
        {
            self.update_bound_to_bottom();
            self.logs.process_event(event);
            return ResponseEvent::Handled;
        } else if self.logs.process_event(event) == ResponseEvent::Handled {
            self.update_bound_to_bottom();
            return ResponseEvent::Handled;
        }

        ResponseEvent::NotHandled
    }

    fn draw(&mut self, frame: &mut Frame<'_>, area: Rect) {
        self.logs.draw(frame, area, self.get_offset());
        self.command_palette.draw(frame, frame.area());
        self.search.draw(frame, frame.area());

        if area.height != self.area.height && self.bound_to_bottom {
            self.area = area;
            self.logs.scroll_to_end();
        }
    }
}

impl Drop for LogsView {
    fn drop(&mut self) {
        self.observer.stop();
    }
}
