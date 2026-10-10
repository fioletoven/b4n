use b4n_kube::{Kind, Namespace};
use b4n_tui::widgets::Spinner;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::core::ConnectionState;
use crate::{core::SharedAppData, ui::presentation::utils::get_right_breadcrumbs};

/// Header pane that shows resource namespace, kind and name.
#[derive(Default)]
pub struct ContentHeader {
    pub title: String,
    pub icon: char,
    pub namespace: Namespace,
    pub kind: Kind,
    pub name: Option<String>,
    pub descr: Option<String>,
    app_data: SharedAppData,
    name_icon: char,
    edit_icon: char,
    edit_mode: &'static str,
    show_coordinates: bool,
    position_x: usize,
    position_y: usize,
    is_busy: bool,
    spinner: Spinner,
    path_cache: Option<Paragraph<'static>>,
}

impl ContentHeader {
    /// Creates new UI header pane.
    pub fn new(app_data: SharedAppData, show_coordinates: bool) -> Self {
        Self {
            icon: ' ',
            namespace: Namespace::all(),
            app_data,
            name_icon: ' ',
            edit_icon: ' ',
            show_coordinates,
            spinner: Spinner::default(),
            ..Default::default()
        }
    }

    /// Sets header data.
    pub fn set_data(&mut self, namespace: Namespace, kind: Kind, name: Option<String>, descr: Option<String>) {
        self.namespace = namespace;
        self.kind = kind;
        self.name = name;
        self.descr = descr;
        self.path_cache = None;
    }

    /// Sets header title.
    pub fn set_title(&mut self, title: impl Into<String>) {
        self.title = title.into();
        self.path_cache = None;
    }

    /// Sets header icon.
    pub fn set_icon(&mut self, icon: char) {
        self.icon = icon;
        self.path_cache = None;
    }

    /// Sets name icon.
    pub fn set_name_icon(&mut self, icon: char) {
        self.name_icon = icon;
        self.path_cache = None;
    }

    /// Sets header coordinates.
    pub fn set_coordinates(&mut self, x: usize, y: usize) {
        self.show_coordinates = true;
        self.position_x = x + 1;
        self.position_y = y + 1;
    }

    /// Hides header coordinates.
    pub fn hide_coordinates(&mut self) {
        self.show_coordinates = false;
    }

    /// Sets edit icon and mode text.
    pub fn set_edit(&mut self, icon: char, mode: &'static str) {
        self.edit_icon = icon;
        self.edit_mode = mode;
        self.path_cache = None;
    }

    /// Sets busy flag.
    pub fn set_busy(&mut self, is_busy: bool) {
        self.is_busy = is_busy;
    }

    /// Draws [`ContentHeader`] on the provided frame area.
    pub fn draw(&mut self, frame: &mut ratatui::Frame<'_>, area: Rect) {
        let coordinates = if self.app_data.borrow().state == ConnectionState::Ready && !self.is_busy {
            format!("  {}Ln {}, Col {} ", self.edit_mode, self.position_y, self.position_x)
        } else {
            format!(
                " {} {}Ln {}, Col {} ",
                self.spinner.tick(),
                self.edit_mode,
                self.position_y,
                self.position_x
            )
        };

        let coordinates_len = u16::try_from(coordinates.chars().count() + 2).unwrap_or_default();
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![
                Constraint::Fill(1),
                Constraint::Length(if self.show_coordinates { coordinates_len } else { 0 }),
            ])
            .split(area);

        if self.path_cache.is_none() {
            self.path_cache = Some(self.build_path());
        }

        if let Some(path) = self.path_cache.as_ref() {
            frame.render_widget(path, layout[0]);
        }

        if self.show_coordinates {
            let text = &self.app_data.borrow().theme.colors.text;
            frame.render_widget(Paragraph::new(self.get_right_text(coordinates)).style(text), layout[1]);
        }
    }

    /// Builds formatted header path as breadcrumbs:\
    /// \> `title` \[`icon`\] \> `namespace` \> `kind` \> `name` \> \[ `descr` \> \]
    fn build_path(&self) -> Paragraph<'static> {
        let bg = self.app_data.borrow().theme.colors.text.bg;
        let colors = &self.app_data.borrow().theme.colors.header;
        let icons = &self.app_data.borrow().config.symbols;

        let title = if self.icon == ' ' && self.edit_icon == ' ' {
            format!(" {} ", self.title)
        } else if self.edit_icon != ' ' {
            format!(" {} {} ", self.title, self.edit_icon)
        } else {
            format!(" {} {} ", self.title, self.icon)
        };

        let mut path = vec![
            Span::styled(&icons.left_end, Style::new().fg(colors.text.bg).bg(bg)),
            Span::styled(title, &colors.text),
            Span::styled(&icons.left_sep, Style::new().fg(colors.text.bg).bg(colors.namespace.bg)),
            Span::styled(format!(" {} ", self.namespace.as_str().to_lowercase()), &colors.namespace),
            Span::styled(&icons.left_sep, Style::new().fg(colors.namespace.bg).bg(colors.resource.bg)),
            Span::styled(format!(" {} ", self.kind.name().to_lowercase()), &colors.resource),
        ];

        let mut end_bg_color = colors.resource.bg;
        if let Some(name) = &self.name {
            let has_icon = self.descr.is_none() && self.name_icon != ' ';
            let name = if has_icon {
                format!(" {} {} ", name.to_lowercase(), self.name_icon)
            } else {
                format!(" {} ", name.to_lowercase())
            };
            path.append(&mut vec![
                Span::styled(&icons.left_sep, Style::new().fg(colors.resource.bg).bg(colors.name.bg)),
                Span::styled(name, &colors.name),
            ]);
            end_bg_color = colors.name.bg;
        }

        if let Some(descr) = &self.descr {
            let descr = if self.name_icon == ' ' {
                format!(" {descr} ")
            } else {
                format!(" {descr} {} ", self.name_icon)
            };
            path.append(&mut vec![
                Span::styled(&icons.left_sep, Style::new().fg(end_bg_color).bg(colors.count.bg)),
                Span::styled(descr, &colors.count),
                Span::styled(&icons.left_sep, Style::new().fg(colors.count.bg).bg(bg)),
            ]);
        } else {
            path.push(Span::styled(&icons.left_sep, Style::new().fg(end_bg_color).bg(bg)));
        }

        Paragraph::new(Line::from(path)).style(&self.app_data.borrow().theme.colors.text)
    }

    /// Returns formatted text as right breadcrumbs:\
    /// \< `text` \<
    fn get_right_text(&self, text: String) -> Line<'_> {
        let colors = if self.app_data.borrow().is_connected() {
            &self.app_data.borrow().theme.colors.header.text
        } else {
            &self.app_data.borrow().theme.colors.header.disconnected
        };

        get_right_breadcrumbs(
            text,
            &self.app_data.borrow().config.symbols,
            colors,
            self.app_data.borrow().theme.colors.text.bg,
        )
    }
}
