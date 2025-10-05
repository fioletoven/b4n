use ratatui::{Frame, layout::Rect};
use std::{collections::HashSet, rc::Rc};
use tokio::sync::{mpsc::UnboundedSender, oneshot::Receiver};

use crate::{
    core::{
        HighlightError, HighlightRequest, HighlightResponse, SharedAppData, SharedAppDataExt, SharedBgWorker,
        commands::CommandResult,
    },
    kubernetes::{ResourceRef, resources::SECRETS},
    ui::{
        KeyCommand, MouseEventKind, ResponseEvent, Responsive, TuiEvent,
        views::{
            View,
            content::{Content, ContentViewer, StyledLine},
            content_search::MatchPosition,
        },
        widgets::{ActionItem, ActionsListBuilder, CommandPalette, FooterTx, IconKind, Search},
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
            footer,
        }
    }

    fn copy_yaml_to_clipboard(&self) {
        if self.yaml.content().is_some() {
            if let Some(clipboard) = &mut self.app_data.borrow_mut().clipboard
                && clipboard
                    .set_text(self.yaml.content().map(|c| c.plain.join("")).unwrap_or_default())
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
}

impl View for YamlView {
    fn command_id(&self) -> Option<&str> {
        self.command_id.as_deref()
    }

    fn process_tick(&mut self) -> ResponseEvent {
        self.yaml.process_tick()
    }

    fn process_command_result(&mut self, result: CommandResult) {
        if let CommandResult::ResourceYaml(Ok(result)) = result {
            let icon = if result.is_decoded { '' } else { '' };
            self.is_decoded = result.is_decoded;
            self.yaml.header.set_icon(icon);
            self.yaml.header.set_data(result.namespace, result.kind, result.name, None);
            let max_width = result.yaml.iter().map(|l| l.chars().count()).max().unwrap_or(0);
            let lowercase = result.yaml.iter().map(|l| l.to_ascii_lowercase()).collect();
            self.yaml.set_content(
                YamlContent {
                    styled: result.styled,
                    plain: result.yaml,
                    lowercase,
                    highlighter: self.worker.borrow().get_higlighter(),
                    modified: HashSet::new(),
                    requested: None,
                },
                max_width,
            );
        }
    }

    fn process_disconnection(&mut self) {
        self.command_palette.hide();
    }

    fn process_event(&mut self, event: &TuiEvent) -> ResponseEvent {
        if self.command_palette.is_visible {
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
            }

            return response;
        }

        if self.search.is_visible {
            let result = self.search.process_event(event);
            if self.yaml.search(self.search.value(), false) {
                self.yaml.scroll_to_current_match(None);
                self.update_search_count();
            }

            return result;
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
            return ResponseEvent::Cancelled;
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
    }
}

/// Number of lines before and after the modified section to include in the re-highlighting process.
const HIGHLIGHT_CONTEXT_LINES_NO: usize = 200;

/// Styled YAML content.
struct YamlContent {
    pub styled: Vec<StyledLine>,
    pub plain: Vec<String>,
    pub lowercase: Vec<String>,
    highlighter: UnboundedSender<HighlightRequest>,
    modified: HashSet<usize>,
    requested: Option<RequestedHighlight>,
}

impl Content for YamlContent {
    fn page(&mut self, start: usize, count: usize) -> &[StyledLine] {
        if start >= self.styled.len() {
            &[]
        } else if start + count >= self.styled.len() {
            &self.styled[start..]
        } else {
            &self.styled[start..start + count]
        }
    }

    fn len(&self) -> usize {
        self.styled.len()
    }

    fn search(&self, pattern: &str) -> Vec<MatchPosition> {
        let pattern = pattern.to_ascii_lowercase();
        let mut matches = Vec::new();
        for (y, line) in self.lowercase.iter().enumerate() {
            for (x, _) in line.match_indices(&pattern) {
                matches.push(MatchPosition::new(x, y, pattern.len()));
            }
        }

        matches
    }

    fn line_size(&self, line_no: usize) -> usize {
        self.plain.get(line_no).map(|l| l.chars().count()).unwrap_or_default()
    }

    fn is_editable(&self) -> bool {
        true
    }

    fn insert_char(&mut self, x: usize, y: usize, character: char) {
        self.plain[y].insert(x, character);
        styled_insert(&mut self.styled[y], x, character);
        self.modified.insert(y);
    }

    fn process_tick(&mut self) -> ResponseEvent {
        if let Some(requested) = &mut self.requested
            && let Ok(response) = requested.response.try_recv()
        {
            if self.modified.is_empty()
                && let Ok(response) = response
            {
                self.styled.splice(requested.start..=requested.end, response.styled);
            }

            self.requested = None;
        }

        if self.requested.is_none() && !self.modified.is_empty() {
            let first = self.modified.iter().min().copied().unwrap_or_default();
            let last = self.modified.iter().max().copied().unwrap_or_default();
            let start = first.saturating_sub(HIGHLIGHT_CONTEXT_LINES_NO);
            let end = last
                .saturating_add(HIGHLIGHT_CONTEXT_LINES_NO)
                .min(self.plain.len().saturating_sub(1));

            let (tx, rx) = tokio::sync::oneshot::channel();

            let _ = self.highlighter.send(HighlightRequest::Partial {
                start: first.saturating_sub(start),
                lines: self.plain[start..=end].to_vec(),
                response: tx,
            });

            self.modified.clear();
            self.requested = Some(RequestedHighlight {
                start: first,
                end,
                response: rx,
            });
        }

        ResponseEvent::Handled
    }
}

struct RequestedHighlight {
    pub start: usize,
    pub end: usize,
    pub response: Receiver<Result<HighlightResponse, HighlightError>>,
}

fn styled_insert(line: &mut StyledLine, x: usize, c: char) {
    let mut current = 0;
    for part in line {
        if current + part.1.len() >= x {
            part.1.insert(x - current, c);
            return;
        }

        current += part.1.len();
    }
}
