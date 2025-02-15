use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::app::SharedAppData;

/// Footer widget.
pub struct Footer {
    app_data: SharedAppData,
    version: String,
}

impl Footer {
    /// Creates new UI footer pane.
    pub fn new(app_data: SharedAppData) -> Self {
        let version = format!(" {} v{} ", env!("CARGO_CRATE_NAME"), env!("CARGO_PKG_VERSION"));
        Footer { app_data, version }
    }

    /// Draws [`FooterPane`] on the provided frame area.
    pub fn draw(&mut self, frame: &mut ratatui::Frame<'_>, area: Rect) {
        let footer = self.get_footer(area.width.into());

        frame.render_widget(Paragraph::new(footer), area);
    }

    /// Returns formatted footer line.
    fn get_footer<'a>(&self, terminal_width: usize) -> Line<'a> {
        let footer = format!(" {1:<0$}", terminal_width - 3, &self.version);
        let colors = &self.app_data.borrow().config.theme.colors;

        Line::from(vec![
            Span::styled("", Style::new().fg(colors.header.bg)),
            Span::styled(footer, Style::new().fg(colors.header.fg).bg(colors.header.bg)),
            Span::styled("", Style::new().fg(colors.header.bg)),
        ])
    }
}
