use clipboard::{ClipboardContext, ClipboardProvider};
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{Frame, layout::Rect};
use std::rc::Rc;
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    core::{SharedAppData, SharedBgWorker, commands::CommandResult},
    kubernetes::{ResourceRef, resources::SECRETS},
    ui::{
        ResponseEvent, Responsive, TuiEvent,
        views::{
            View,
            content::{Content, ContentViewer, StyledLine},
            content_search::MatchPosition,
        },
        widgets::{ActionItem, ActionsListBuilder, CommandPalette, FooterIcon, FooterIconAction, FooterMessage, Search},
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
    messages_tx: UnboundedSender<FooterMessage>,
    icons_tx: UnboundedSender<FooterIconAction>,
}

impl YamlView {
    /// Creates new [`YamlView`] instance.
    pub fn new(
        app_data: SharedAppData,
        worker: SharedBgWorker,
        command_id: Option<String>,
        resource: ResourceRef,
        messages_tx: UnboundedSender<FooterMessage>,
        icons_tx: UnboundedSender<FooterIconAction>,
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
            messages_tx,
            icons_tx,
        }
    }

    fn copy_yaml_to_clipboard(&self) {
        let result: Result<ClipboardContext, _> = ClipboardProvider::new();
        if let Ok(mut ctx) = result
            && ctx
                .set_contents(self.yaml.content().map(|c| c.plain.join("")).unwrap_or_default())
                .is_ok()
        {
            self.messages_tx
                .send(FooterMessage::info(" YAML content copied to clipboard…", 1_500))
                .unwrap();
        }
    }

    fn process_command_palette_events(&mut self, key: crossterm::event::KeyEvent) -> bool {
        if key.code == KeyCode::Char(':') || key.code == KeyCode::Char('>') {
            let mut builder = ActionsListBuilder::default().with_close().with_quit().with_action(
                ActionItem::new("copy")
                    .with_description("copies YAML to the clipboard")
                    .with_response(ResponseEvent::Action("copy")),
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
            true
        } else {
            false
        }
    }

    fn toggle_yaml_decode(&mut self) {
        self.command_id = self.worker.borrow_mut().get_yaml(
            self.yaml.header.name.clone(),
            self.yaml.header.namespace.clone(),
            &self.yaml.header.kind,
            self.app_data.borrow().get_syntax_data(),
            !self.is_decoded,
        );
    }

    fn clear_search(&mut self) {
        self.yaml.search("");
        self.search.reset();
        self.update_search_count();
    }

    fn update_search_count(&mut self) {
        let count = self.yaml.search_matches_count();
        self.set_footer_icon(count);
        self.search.set_matches(count);
    }

    fn set_footer_icon(&self, count: Option<usize>) {
        if let Some(count) = count {
            let icon = FooterIcon::text("yaml_search", format!(" {count}"));
            let _ = self.icons_tx.send(FooterIconAction::Add(icon));
        } else {
            let _ = self.icons_tx.send(FooterIconAction::Remove("yaml_search"));
        };
    }
}

impl View for YamlView {
    fn command_id(&self) -> Option<&str> {
        self.command_id.as_deref()
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
                },
                max_width,
            );
        }
    }

    fn process_disconnection(&mut self) {
        self.command_palette.hide();
    }

    fn process_event(&mut self, event: TuiEvent) -> ResponseEvent {
        let TuiEvent::Key(key) = event;

        if key.code == KeyCode::Char('c') && key.modifiers == KeyModifiers::CONTROL {
            return ResponseEvent::ExitApplication;
        }

        if self.command_palette.is_visible {
            let response = self.command_palette.process_key(key);
            if response == ResponseEvent::Cancelled {
                self.clear_search();
            } else if response.is_action("copy") {
                self.copy_yaml_to_clipboard();
                return ResponseEvent::Handled;
            } else if response.is_action("decode") {
                self.toggle_yaml_decode();
                return ResponseEvent::Handled;
            }

            return response;
        }

        if self.search.is_visible {
            let result = self.search.process_key(key);
            if self.yaml.search(self.search.value()) {
                self.update_search_count();
            }

            return result;
        }

        if self.process_command_palette_events(key) {
            return ResponseEvent::Handled;
        }

        if key.code == KeyCode::Char('/') {
            self.search.show();
        }

        if key.code == KeyCode::Char('x') && self.yaml.header.kind.as_str() == SECRETS && self.app_data.borrow().is_connected {
            self.toggle_yaml_decode();
            self.clear_search();
            return ResponseEvent::Handled;
        }

        if key.code == KeyCode::Char('c') {
            self.copy_yaml_to_clipboard();
            return ResponseEvent::Handled;
        }

        if key.code == KeyCode::Char('n') && self.yaml.search_matches_count().is_some() {
            self.yaml.navigate_match(true);
        }

        if key.code == KeyCode::Char('p') && self.yaml.search_matches_count().is_some() {
            self.yaml.navigate_match(false);
        }

        if key.code == KeyCode::Esc {
            if self.search.value().is_empty() {
                return ResponseEvent::Cancelled;
            } else {
                self.clear_search();
                return ResponseEvent::Handled;
            }
        }

        self.yaml.process_key(key)
    }

    fn draw(&mut self, frame: &mut Frame<'_>, area: Rect) {
        self.yaml.draw(frame, area);
        self.command_palette.draw(frame, frame.area());
        self.search.draw(frame, frame.area());
    }
}

/// Styled YAML content.
struct YamlContent {
    pub styled: Vec<StyledLine>,
    pub plain: Vec<String>,
    pub lowercase: Vec<String>,
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
}
