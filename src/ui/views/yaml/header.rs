use ratatui::{
    layout::Rect,
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
    app_data: SharedAppData,
}

impl HeaderPane {
    /// Creates new UI header pane.
    pub fn new(app_data: SharedAppData, name: String, namespace: Namespace, kind_plural: String) -> Self {
        Self {
            name,
            namespace,
            kind_plural,
            app_data,
        }
    }

    /// Sets header data.
    pub fn set_data(&mut self, name: String, namespace: Namespace, kind_plural: String) {
        self.name = name;
        self.namespace = namespace;
        self.kind_plural = kind_plural;
    }

    /// Draws [`HeaderPane`] on the provided frame area.
    pub fn draw(&mut self, frame: &mut ratatui::Frame<'_>, area: Rect) {
        frame.render_widget(Paragraph::new(self.get_path()), area);
    }

    /// Returns formatted YAML resource path as breadcrumbs:  
    /// \> `YAML` \> `namespace` \> `kind` \> `name` \>
    fn get_path(&self) -> Line {
        let colors = &self.app_data.borrow().config.theme.colors;
        let path = vec![
            Span::styled("", Style::new().fg(colors.header.bg)),
            Span::styled(" YAML  ", Style::new().fg(colors.header.fg).bg(colors.header.bg)),
            Span::styled("", Style::new().fg(colors.header.bg).bg(colors.namespace.bg)),
            Span::styled(
                format!(" {} ", self.namespace.as_str().to_lowercase()),
                Style::new().fg(colors.namespace.fg).bg(colors.namespace.bg),
            ),
            Span::styled("", Style::new().fg(colors.namespace.bg).bg(colors.resource.bg)),
            Span::styled(
                format!(" {} ", self.kind_plural.to_lowercase()),
                Style::new().fg(colors.resource.fg).bg(colors.resource.bg),
            ),
            Span::styled("", Style::new().fg(colors.resource.bg).bg(colors.count.bg)),
            Span::styled(
                format!(" {} ", self.name.to_lowercase()),
                Style::new().fg(colors.count.fg).bg(colors.count.bg),
            ),
            Span::styled("", Style::new().fg(colors.count.bg)),
        ];

        Line::from(path)
    }
}
