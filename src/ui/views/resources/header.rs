use kube::discovery::Scope;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::{
    app::SharedAppData,
    kubernetes::resources::{CONTAINERS, PODS},
};

/// Header pane that shows resource path and version information as breadcrumbs.
pub struct HeaderPane {
    app_data: SharedAppData,
    is_filtered: bool,
}

impl HeaderPane {
    /// Creates new UI header pane.
    pub fn new(app_data: SharedAppData) -> Self {
        Self {
            app_data,
            is_filtered: false,
        }
    }

    /// Draws [`HeaderPane`] on the provided frame area.
    pub fn draw(&mut self, frame: &mut ratatui::Frame<'_>, area: Rect) {
        let path = self.get_path();
        let version = self.get_version();

        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Fill(1), Constraint::Length(version.width() as u16)])
            .split(area);

        frame.render_widget(Paragraph::new(path), layout[0]);
        frame.render_widget(Paragraph::new(version), layout[1]);
    }

    /// Sets if header should show icon that indicates data is filtered.
    pub fn show_filtered_icon(&mut self, is_filtered: bool) {
        self.is_filtered = is_filtered;
    }

    /// Returns formatted kubernetes resource path as breadcrumbs:\
    /// \> `context name` \> \[ `namespace` \> \] `resource` \> `resources count` \>
    fn get_path(&self) -> Line {
        let colors = &self.app_data.borrow().theme.colors.header;
        let data = &self.app_data.borrow().current;
        let mut path = vec![
            Span::styled("", Style::new().fg(colors.context.bg)),
            Span::styled(format!(" {} ", data.context), &colors.context),
        ];

        if data.scope == Scope::Namespaced {
            path.append(&mut vec![
                Span::styled("", Style::new().fg(colors.context.bg).bg(colors.namespace.bg)),
                Span::styled(format!(" {} ", data.namespace.as_str()), &colors.namespace),
                Span::styled("", Style::new().fg(colors.namespace.bg).bg(colors.resource.bg)),
            ]);
        } else {
            path.push(Span::styled("", Style::new().fg(colors.context.bg).bg(colors.resource.bg)));
        }

        let count_icon = if self.is_filtered {
            ""
        } else if data.name.is_some() {
            ""
        } else {
            ""
        };

        let kind = if data.kind.name() == CONTAINERS {
            PODS
        } else {
            data.kind.name()
        };

        path.push(Span::styled(format!(" {kind} "), &colors.resource));
        if data.name.is_some() {
            path.append(&mut vec![
                Span::styled("", Style::new().fg(colors.resource.bg).bg(colors.name.bg)),
                Span::styled(format!(" {} ", data.name.as_ref().unwrap()), &colors.name),
                Span::styled("", Style::new().fg(colors.name.bg).bg(colors.count.bg)),
            ]);
        } else {
            path.push(Span::styled("", Style::new().fg(colors.resource.bg).bg(colors.count.bg)));
        }

        path.append(&mut vec![
            Span::styled(format!(" {}{} ", count_icon, data.count), &colors.count),
            Span::styled("", Style::new().fg(colors.count.bg)),
        ]);

        Line::from(path)
    }

    /// Returns formatted k8s version info as breadcrumbs:\
    /// \< `k8s version` \<
    fn get_version(&self) -> Line {
        let data = &self.app_data.borrow().current;
        let colors;
        let text;
        if self.app_data.borrow().is_connected {
            colors = self.app_data.borrow().theme.colors.header.info;
            text = format!(" {} ", &data.version);
        } else {
            colors = self.app_data.borrow().theme.colors.header.disconnected;
            text = format!(
                "  {} ",
                if data.version.is_empty() {
                    "connecting…"
                } else {
                    &data.version
                }
            );
        }

        Line::from(vec![
            Span::styled("", Style::new().fg(colors.bg)),
            Span::styled(text, &colors),
            Span::styled("", Style::new().fg(colors.bg)),
        ])
        .right_aligned()
    }
}
