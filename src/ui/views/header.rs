use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::{
    core::SharedAppData,
    kubernetes::{Kind, Namespace},
};

/// Header pane that shows resource namespace, kind and name.
pub struct HeaderPane {
    pub title: &'static str,
    pub icon: char,
    pub namespace: Namespace,
    pub kind: Kind,
    pub name: String,
    pub descr: Option<String>,
    app_data: SharedAppData,
    show_coordinates: bool,
    position_x: usize,
    position_y: usize,
}

impl HeaderPane {
    /// Creates new UI header pane.
    pub fn new(app_data: SharedAppData, show_coordinates: bool) -> Self {
        Self {
            title: "",
            icon: ' ',
            namespace: Namespace::all(),
            kind: Kind::default(),
            name: String::new(),
            descr: None,
            app_data,
            show_coordinates,
            position_x: 0,
            position_y: 0,
        }
    }

    /// Sets header data.
    pub fn set_data(&mut self, namespace: Namespace, kind: Kind, name: String, descr: Option<String>) {
        self.namespace = namespace;
        self.kind = kind;
        self.name = name;
        self.descr = descr;
    }

    /// Sets header title.
    pub fn set_title(&mut self, title: &'static str) {
        self.title = title;
    }

    /// Sets header icon.
    pub fn set_icon(&mut self, icon: char) {
        self.icon = icon;
    }

    /// Sets header coordinates.
    pub fn set_coordinates(&mut self, x: usize, y: usize) {
        self.show_coordinates = true;
        self.position_x = x;
        self.position_y = y;
    }

    /// Draws [`HeaderPane`] on the provided frame area.
    pub fn draw(&mut self, frame: &mut ratatui::Frame<'_>, area: Rect) {
        let coordinates = if self.app_data.borrow().is_connected {
            format!("  Ln {}, Col {} ", self.position_y, self.position_x)
        } else {
            format!("  Ln {}, Col {} ", self.position_y, self.position_x)
        };

        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![
                Constraint::Fill(1),
                Constraint::Length(coordinates.chars().count() as u16 + 2),
            ])
            .split(area);

        frame.render_widget(Paragraph::new(self.get_path()), layout[0]);
        if self.show_coordinates {
            frame.render_widget(Paragraph::new(self.get_right_text(coordinates)), layout[1]);
        }
    }

    /// Returns formatted header path as breadcrumbs:\
    /// \> `title` \[`icon`\] \> `namespace` \> `kind` \> `name` \> \[ `descr` \> \]
    fn get_path(&self) -> Line {
        let colors = &self.app_data.borrow().theme.colors.header;
        let title = if self.icon != ' ' {
            format!(" {} {} ", self.title, self.icon)
        } else {
            format!(" {} ", self.title)
        };

        let mut path = vec![
            Span::styled("", Style::new().fg(colors.text.bg)),
            Span::styled(title, &colors.text),
            Span::styled("", Style::new().fg(colors.text.bg).bg(colors.namespace.bg)),
            Span::styled(format!(" {} ", self.namespace.as_str().to_lowercase()), &colors.namespace),
            Span::styled("", Style::new().fg(colors.namespace.bg).bg(colors.resource.bg)),
            Span::styled(format!(" {} ", self.kind.as_str().to_lowercase()), &colors.resource),
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

    /// Returns formatted text as right breadcrumbs:\
    /// \< `text` \<
    fn get_right_text(&self, text: String) -> Line {
        let colors = if self.app_data.borrow().is_connected {
            &self.app_data.borrow().theme.colors.header.text
        } else {
            &self.app_data.borrow().theme.colors.header.disconnected
        };

        crate::ui::views::get_right_breadcrumbs(text, colors)
    }
}
