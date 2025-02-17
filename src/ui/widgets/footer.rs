use std::time::Instant;

use ratatui::{
    layout::{Constraint, Flex, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::app::SharedAppData;

/// Footer widget.
pub struct Footer {
    app_data: SharedAppData,
    version: String,
    message: Option<String>,
    duration: u16,
    start: Instant,
}

impl Footer {
    /// Creates new UI footer pane.
    pub fn new(app_data: SharedAppData) -> Self {
        let version = format!(" {} v{} ", env!("CARGO_CRATE_NAME"), env!("CARGO_PKG_VERSION"));
        Footer {
            app_data,
            version,
            message: None,
            duration: 0,
            start: Instant::now(),
        }
    }

    /// Shows `message` for the `duration` of milliseconds.
    pub fn show_message(&mut self, message: String, duration: u16) {
        self.message = Some(message);
        self.duration = duration;
        self.start = Instant::now();
    }

    /// Draws [`Footer`] on the provided frame area.
    pub fn draw(&mut self, frame: &mut ratatui::Frame<'_>, area: Rect) {
        let footer = self.get_footer(area.width.into());
        frame.render_widget(Paragraph::new(footer), area);

        if self.has_message_to_show() {
            if let Some(message) = &self.message {
                let [area] = Layout::horizontal([Constraint::Length(message.chars().count() as u16)])
                    .flex(Flex::Center)
                    .areas(area);
                frame.render_widget(self.get_message(message), area);
            }
        }
    }

    /// Returns formatted footer line.
    fn get_footer(&self, terminal_width: usize) -> Line<'_> {
        let footer = format!(" {1:<0$}", terminal_width - 3, &self.version);
        let colors = &self.app_data.borrow().config.theme.colors;

        Line::from(vec![
            Span::styled("", Style::new().fg(colors.footer.bg)),
            Span::styled(footer, Style::new().fg(colors.footer.fg).bg(colors.footer.bg)),
            Span::styled("", Style::new().fg(colors.footer.bg)),
        ])
    }

    /// Returns formatted message to show.
    fn get_message<'a>(&self, message: &'a str) -> Line<'a> {
        let colors = &self.app_data.borrow().config.theme.colors;
        Line::styled(message, Style::new().fg(colors.footer.dim).bg(colors.footer.bg))
    }

    /// Returns `true` if there is a message to show.
    fn has_message_to_show(&mut self) -> bool {
        if self.message.is_some() {
            if self.start.elapsed().as_millis() <= self.duration.into() {
                true
            } else {
                self.message = None;
                false
            }
        } else {
            false
        }
    }
}
