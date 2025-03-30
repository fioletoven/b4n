use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::{app::SharedAppData, kubernetes::Namespace};

/// Header pane that shows resource namespace, kind and name.
pub struct HeaderPane {
    pub title: &'static str,
    pub namespace: Namespace,
    pub kind: String,
    pub name: String,
    pub descr: Option<String>,
    app_data: SharedAppData,
    position_x: usize,
    position_y: usize,
}

impl HeaderPane {
    /// Creates new UI header pane.
    pub fn new(
        app_data: SharedAppData,
        title: &'static str,
        namespace: Namespace,
        kind: String,
        name: String,
        descr: Option<String>,
    ) -> Self {
        Self {
            title,
            namespace,
            kind,
            name,
            descr,
            app_data,
            position_x: 0,
            position_y: 0,
        }
    }

    /// Sets header data.
    pub fn set_data(&mut self, title: &'static str, namespace: Namespace, kind: String, name: String, descr: Option<String>) {
        self.title = title;
        self.namespace = namespace;
        self.kind = kind;
        self.name = name;
        self.descr = descr;
    }

    /// Sets header title.
    pub fn set_title(&mut self, title: &'static str) {
        self.title = title;
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

    /// Returns formatted header path as breadcrumbs:  
    /// \> `title` \> `namespace` \> `kind` \> `name` \> \[ `descr` \> \]
    fn get_path(&self) -> Line {
        let colors = &self.app_data.borrow().theme.colors.header;
        let mut path = vec![
            Span::styled("", Style::new().fg(colors.text.bg)),
            Span::styled(self.title, &colors.text),
            Span::styled("", Style::new().fg(colors.text.bg).bg(colors.namespace.bg)),
            Span::styled(format!(" {} ", self.namespace.as_str().to_lowercase()), &colors.namespace),
            Span::styled("", Style::new().fg(colors.namespace.bg).bg(colors.resource.bg)),
            Span::styled(format!(" {} ", self.kind.to_lowercase()), &colors.resource),
            Span::styled("", Style::new().fg(colors.resource.bg).bg(colors.name.bg)),
            Span::styled(format!(" {} ", self.name.to_lowercase()), &colors.name),
        ];

        if self.descr.is_some() {
            path.append(&mut vec![
                Span::styled("", Style::new().fg(colors.name.bg).bg(colors.count.bg)),
                Span::styled(format!(" {} ", self.descr.as_ref().unwrap()), &colors.count),
                Span::styled("", Style::new().fg(colors.count.bg)),
            ]);
        } else {
            path.push(Span::styled("", Style::new().fg(colors.name.bg)));
        }

        Line::from(path)
    }

    /// Returns formatted text as right breadcrumbs:  
    /// \< `text` \<
    fn get_right_text(&self, text: String) -> Line {
        let colors = &self.app_data.borrow().theme.colors.header;

        Line::from(vec![
            Span::styled("", Style::new().fg(colors.text.bg)),
            Span::styled(text, &colors.text),
            Span::styled("", Style::new().fg(colors.text.bg)),
        ])
        .right_aligned()
    }
}
