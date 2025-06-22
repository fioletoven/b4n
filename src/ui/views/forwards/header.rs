use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::Line,
    widgets::Paragraph,
};

use crate::{core::SharedAppData, kubernetes::resources::PODS};

/// Header pane that shows context, namespace and number of port forwards as breadcrumbs.
pub struct HeaderPane {
    app_data: SharedAppData,
    count: usize,
    is_filtered: bool,
}

impl HeaderPane {
    /// Creates new UI header pane.
    pub fn new(app_data: SharedAppData, count: usize) -> Self {
        Self {
            app_data,
            count,
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

    /// Sets new value for the header count.
    pub fn set_count(&mut self, count: usize) {
        self.count = count;
    }

    /// Sets if header should show icon that indicates data is filtered.
    pub fn show_filtered_icon(&mut self, is_filtered: bool) {
        self.is_filtered = is_filtered;
    }

    /// Returns formatted port forwards path as breadcrumbs:\
    /// \> `context name` \> \[ `namespace` \> \] `pods` \> `port forwards` \> `count` \>
    fn get_path(&self) -> Line {
        let data = &self.app_data.borrow();
        crate::ui::views::get_left_breadcrumbs(data, PODS, Some("port forwards"), self.count, self.is_filtered)
    }

    /// Returns formatted k8s version info as breadcrumbs:\
    /// \< `k8s version` \<
    fn get_version(&self) -> Line {
        let data = &self.app_data.borrow();
        let (text, colors) = crate::ui::views::get_version_text(data);
        crate::ui::views::get_right_breadcrumbs(text, colors)
    }
}
