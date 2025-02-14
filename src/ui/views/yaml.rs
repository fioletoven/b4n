use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};
use syntect::easy::HighlightLines;

use crate::{
    app::AppData,
    ui::{colors::from_syntect_color, ResponseEvent, TuiEvent},
};

use super::View;

pub struct YamlView {
    lines: Vec<Vec<(Style, String)>>,
    page_start: usize,
    page_height: usize,
}

impl YamlView {
    pub fn new(app_data: &AppData, yaml: Vec<String>) -> Self {
        let lines = highlight_lines(app_data, &yaml);

        Self {
            lines,
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
            .map(|items| Line::from(items.iter().map(|item| Span::styled(&item.1, item.0)).collect::<Vec<_>>()))
            .collect::<Vec<_>>();

        frame.render_widget(Paragraph::new(lines), frame.area());
    }
}

fn highlight_lines(data: &AppData, lines: &Vec<String>) -> Vec<Vec<(Style, String)>> {
    let syntax = data.syntax_set.find_syntax_by_extension("yaml").unwrap();
    let theme = data.config.theme.build_syntect_yaml_theme();
    let mut h = HighlightLines::new(syntax, &theme);

    lines
        .iter()
        .map(|line| {
            h.highlight_line(line, &data.syntax_set)
                .unwrap()
                .into_iter()
                .map(|segment| (convert_style(segment.0), segment.1.to_owned()))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>()
}

fn convert_style(style: syntect::highlighting::Style) -> Style {
    Style::default()
        .fg(from_syntect_color(style.foreground))
        .bg(from_syntect_color(style.background))
}
