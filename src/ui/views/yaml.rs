use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{text::Line, widgets::Paragraph, Frame};

use crate::ui::{ResponseEvent, TuiEvent};

use super::View;

pub struct YamlView {
    lines: Vec<String>,
    page_start: usize,
    page_height: usize,
}

impl YamlView {
    pub fn new(yaml: String) -> Self {
        Self {
            lines: yaml.lines().map(String::from).collect(),
            page_start: 0,
            page_height: 0,
        }
    }

    pub fn update_page(&mut self, new_height: u16) {
        self.page_height = usize::from(new_height);
    }

    fn max_start(&self) -> usize {
        self.lines.len().saturating_sub(self.page_height)
    }
}

impl View for YamlView {
    fn process_event(&mut self, event: TuiEvent) -> ResponseEvent {
        let TuiEvent::Key(key) = event;

        if key.code == KeyCode::Char('c') && key.modifiers == KeyModifiers::CONTROL {
            return ResponseEvent::ExitApplication;
        }

        if key.code == KeyCode::Esc {
            return ResponseEvent::Cancelled;
        }

        if key.code == KeyCode::Home {
            self.page_start = 0;
        }

        if key.code == KeyCode::PageUp {
            self.page_start = self.page_start.saturating_sub(self.page_height);
        }

        if key.code == KeyCode::Up {
            if self.page_start > 0 {
                self.page_start -= 1;
            }
        }

        if key.code == KeyCode::Down {
            if self.page_start < self.max_start() {
                self.page_start += 1;
            }
        }

        if key.code == KeyCode::PageDown {
            self.page_start += self.page_height;
            if self.page_start > self.max_start() {
                self.page_start = self.max_start();
            }
        }

        if key.code == KeyCode::End {
            self.page_start = self.max_start();
        }

        ResponseEvent::Handled
    }

    fn draw(&mut self, frame: &mut Frame<'_>) {
        self.update_page(frame.area().height);

        let start = self.page_start.clamp(0, self.max_start());
        let lines = self
            .lines
            .iter()
            .skip(start)
            .take(usize::from(frame.area().height))
            .map(|l| Line::from(l.as_str()))
            .collect::<Vec<_>>();

        frame.render_widget(Paragraph::new(lines), frame.area());
    }
}
