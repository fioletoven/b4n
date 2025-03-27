use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::{app::SharedAppData, kubernetes::Namespace};

/// Header pane that shows resource name and namespace.
pub struct HeaderPane {
    pub name: String,
    pub namespace: Namespace,
    pub kind_plural: String,
    pub is_decoded: bool,
    app_data: SharedAppData,
    position_x: usize,
    position_y: usize,
}

impl HeaderPane {
    /// Creates new UI header pane.
    pub fn new(app_data: SharedAppData, name: String, namespace: Namespace, kind_plural: String, is_decoded: bool) -> Self {
        Self {
            name,
            namespace,
            kind_plural,
            is_decoded,
            app_data,
            position_x: 0,
            position_y: 0,
        }
    }

    /// Sets header data.
    pub fn set_data(&mut self, name: String, namespace: Namespace, kind_plural: String, is_decoded: bool) {
        self.name = name;
        self.namespace = namespace;
        self.kind_plural = kind_plural;
        self.is_decoded = is_decoded;
    }

    /// Sets header coordinates.
    pub fn set_coordinates(&mut self, x: usize, y: usize) {
        self.position_x = x;
        self.position_y = y;
    }

    /// Draws [`HeaderPane`] on the provided frame area.
    pub fn draw(&mut self, frame: &mut ratatui::Frame<'_>, area: Rect) {
        let coordinates = format!("  Ln {}, Col {} ", self.position_y, self.position_x);

        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![
                Constraint::Fill(1),
                Constraint::Length(coordinates.chars().count() as u16 + 2),
            ])
            .split(area);

        frame.render_widget(Paragraph::new(self.get_path()), layout[0]);
        frame.render_widget(Paragraph::new(self.get_right_text(coordinates)), layout[1]);
    }

    /// Returns formatted YAML resource path as breadcrumbs:  
    /// \> `YAML` \> `namespace` \> `kind` \> `name` \>
    fn get_path(&self) -> Line {
        let colors = &self.app_data.borrow().theme.colors.header;
        let header_text = if self.is_decoded { " YAML  " } else { " YAML  " };
        let path = vec![
            Span::styled("", Style::new().fg(colors.text.bg)),
            Span::styled(header_text, &colors.text),
            Span::styled("", Style::new().fg(colors.text.bg).bg(colors.namespace.bg)),
            Span::styled(format!(" {} ", self.namespace.as_str().to_lowercase()), &colors.namespace),
            Span::styled("", Style::new().fg(colors.namespace.bg).bg(colors.resource.bg)),
            Span::styled(format!(" {} ", self.kind_plural.to_lowercase()), &colors.resource),
            Span::styled("", Style::new().fg(colors.resource.bg).bg(colors.count.bg)),
            Span::styled(format!(" {} ", self.name.to_lowercase()), &colors.count),
            Span::styled("", Style::new().fg(colors.count.bg)),
        ];

        Line::from(path)
    }

    /// Returns formatted text as right breadcrumbs:  
    /// \< `text` \<
    fn get_right_text(&self, text: String) -> Line {
        let header = &self.app_data.borrow().theme.colors.header;

        Line::from(vec![
            Span::styled("", Style::new().fg(header.text.bg)),
            Span::styled(text, &header.text),
            Span::styled("", Style::new().fg(header.text.bg)),
        ])
        .right_aligned()
    }
}
