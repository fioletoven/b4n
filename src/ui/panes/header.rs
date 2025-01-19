use kube::discovery::Scope;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::app::SharedAppData;

/// Header pane that shows resource path and version information as breadcrumbs.
pub struct HeaderPane {
    app_data: SharedAppData,
}

impl HeaderPane {
    /// Creates new UI header pane.
    pub fn new(app_data: SharedAppData) -> Self {
        Self { app_data }
    }

    /// Draws [`HeaderPane`] on the provided frame area.
    pub fn draw(&mut self, frame: &mut ratatui::Frame<'_>, area: Rect) {
        let data = &self.app_data.borrow().current;
        let path = self.get_path(
            &data.context,
            &data.namespace,
            &data.kind_plural,
            data.count,
            data.scope.clone(),
        );
        let version = self.get_version(&data.version);

        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Fill(1), Constraint::Length(version.width() as u16)])
            .split(area);

        frame.render_widget(Paragraph::new(path), layout[0]);
        frame.render_widget(Paragraph::new(version), layout[1]);
    }

    /// Returns formatted kubernetes resource path as breadcrumbs:
    /// > `context name` [ > `namespace` > ] `resource` > `resources count` >
    fn get_path(&self, context: &str, namespace: &str, resource: &str, count: usize, scope: Scope) -> Line {
        let colors = &self.app_data.borrow().config.theme.colors;
        let mut path = vec![
            Span::styled("", Style::new().fg(colors.context.bg)),
            Span::styled(
                format!(" {} ", context.to_lowercase()),
                Style::default().fg(colors.context.fg).bg(colors.context.bg),
            ),
        ];

        if scope == Scope::Namespaced {
            path.append(&mut vec![
                Span::styled("", Style::new().fg(colors.context.bg).bg(colors.namespace.bg)),
                Span::styled(
                    format!(" {} ", namespace.to_lowercase()),
                    Style::new().fg(colors.namespace.fg).bg(colors.namespace.bg),
                ),
                Span::styled("", Style::new().fg(colors.namespace.bg).bg(colors.resource.bg)),
            ]);
        } else {
            path.push(Span::styled("", Style::new().fg(colors.context.bg).bg(colors.resource.bg)));
        }

        path.append(&mut vec![
            Span::styled(
                format!(" {} ", resource.to_lowercase()),
                Style::new().fg(colors.resource.fg).bg(colors.resource.bg),
            ),
            Span::styled("", Style::new().fg(colors.resource.bg).bg(colors.count.bg)),
            Span::styled(format!(" {} ", count), Style::new().fg(colors.count.fg).bg(colors.count.bg)),
            Span::styled("", Style::new().fg(colors.count.bg)),
        ]);

        Line::from(path)
    }

    /// Returns formatted k8s version info as breadcrumbs:
    /// < `k8s version` <
    fn get_version(&self, version: &str) -> Line {
        let colors;
        let text;
        if self.app_data.borrow().is_connected {
            colors = self.app_data.borrow().config.theme.colors.info;
            text = format!(" {} ", version);
        } else {
            colors = self.app_data.borrow().config.theme.colors.disconnected;
            text = format!("  {} ", if version.is_empty() { "connecting…" } else { version });
        };

        Line::from(vec![
            Span::styled("", Style::new().fg(colors.bg)),
            Span::styled(text, Style::new().fg(colors.fg).bg(colors.bg)),
            Span::styled("", Style::new().fg(colors.bg)),
        ])
        .right_aligned()
    }
}
