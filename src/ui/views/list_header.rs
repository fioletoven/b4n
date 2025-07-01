use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::Line,
    widgets::Paragraph,
};

use crate::{
    core::SharedAppData,
    kubernetes::resources::{CONTAINERS, PODS},
};

/// Header pane that shows context, namespace, kind and number of items as a breadcrumbs.
pub struct ListHeader {
    app_data: SharedAppData,
    fixed_kind: Option<&'static str>,
    count: usize,
    is_filtered: bool,
}

impl ListHeader {
    /// Creates new UI header pane.\
    /// **Note** that setting `fixed_kind` to Some will prevent header from displaying name.
    pub fn new(app_data: SharedAppData, fixed_kind: Option<&'static str>, count: usize) -> Self {
        Self {
            app_data,
            fixed_kind,
            count,
            is_filtered: false,
        }
    }

    /// Draws [`ListHeader`] on the provided frame area.
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

    /// Sets new value for the header count.
    pub fn set_count(&mut self, count: usize) {
        self.count = count;
    }

    /// Sets if header should show icon that indicates data is filtered.
    pub fn show_filtered_icon(&mut self, is_filtered: bool) {
        self.is_filtered = is_filtered;
    }

    /// Returns formatted resource path as breadcrumbs:\
    /// \> `context name` \> \[ `namespace` \> \] `kind` \> \[ `name` \> \] `resources count` \>
    fn get_path(&self) -> Line {
        let data = &self.app_data.borrow();
        let kind = match self.fixed_kind.as_ref() {
            Some(kind) => kind,
            None => {
                if data.current.kind.name() == CONTAINERS {
                    PODS
                } else {
                    data.current.kind.name()
                }
            },
        };
        let name = if self.fixed_kind.is_some() {
            None
        } else {
            data.current.name.as_deref()
        };

        super::get_left_breadcrumbs(data, kind, name, self.count, self.is_filtered)
    }

    /// Returns formatted k8s version info as breadcrumbs:\
    /// \< `k8s version` \<
    fn get_version(&self) -> Line {
        let data = &self.app_data.borrow();
        let (text, colors) = super::get_version_text(data);
        super::get_right_breadcrumbs(text, colors)
    }
}
