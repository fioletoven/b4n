use b4n_config::themes::TextColors;
use b4n_tui::widgets::Spinner;
use kube::discovery::Scope;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::core::{AppData, SharedAppData};
use crate::ui::presentation::utils::{get_left_breadcrumbs, get_right_breadcrumbs};

/// Header pane that shows context, namespace, kind and number of items as a breadcrumbs.
pub struct ListHeader {
    app_data: SharedAppData,
    count: usize,
    fixed_scope: Option<Scope>,
    fixed_kind: Option<&'static str>,
    fixed_namespace: Option<String>,
    is_filtered: bool,
    hide_previous: bool,
    spinner: Spinner,
}

impl ListHeader {
    /// Creates new UI header pane.\
    /// **Note** that setting `fixed_kind` to Some will prevent header from displaying name.
    pub fn new(app_data: SharedAppData, count: usize) -> Self {
        Self {
            app_data,
            count,
            fixed_scope: None,
            fixed_kind: None,
            fixed_namespace: None,
            is_filtered: false,
            hide_previous: false,
            spinner: Spinner::default(),
        }
    }

    /// Sets fixed kind name for the header.
    pub fn with_kind(mut self, kind: &'static str) -> Self {
        self.fixed_kind = Some(kind);
        self
    }

    /// Sets fixed namespace name for the header.
    pub fn with_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.fixed_namespace = Some(namespace.into());
        self
    }

    /// Sets new fixed namespace for the header.
    pub fn set_namespace(&mut self, namespace: Option<impl Into<String>>) {
        self.fixed_namespace = namespace.map(Into::into);
    }

    /// Sets fixed scope for the header.
    pub fn with_scope(mut self, scope: Scope) -> Self {
        self.fixed_scope = Some(scope);
        self
    }

    /// Sets whether the header should hide the "previous" indicator.
    pub fn with_hide_previous(mut self, hide: bool) -> Self {
        self.hide_previous = hide;
        self
    }

    /// Sets new fixed scope for the header.
    pub fn set_scope(&mut self, scope: Option<Scope>) {
        self.fixed_scope = scope;
    }

    /// Returns current scope that header will use.\
    /// **Note** that it borrows app data to do that.
    pub fn get_scope(&self) -> Scope {
        if let Some(scope) = &self.fixed_scope {
            scope.clone()
        } else {
            self.app_data.borrow().current.scope.clone()
        }
    }

    /// Sets new value for the header count.
    pub fn set_count(&mut self, count: usize) {
        self.count = count;
    }

    /// Sets if header should show icon that indicates data is filtered.
    pub fn show_filtered_icon(&mut self, is_filtered: bool) {
        self.is_filtered = is_filtered;
    }

    /// Draws [`ListHeader`] on the provided frame area.
    pub fn draw(&mut self, frame: &mut ratatui::Frame<'_>, area: Rect) {
        let text_style = Style::from(&self.app_data.borrow().theme.colors.text);
        let version = self.get_version();

        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Fill(1), Constraint::Length(version.width() as u16)])
            .split(area);

        frame.render_widget(Paragraph::new(version).style(text_style), layout[1]);

        let path = self.get_path(self.fixed_scope.as_ref());
        frame.render_widget(Paragraph::new(path).style(text_style), layout[0]);
    }

    /// Returns formatted resource path as breadcrumbs:\
    /// \> `context name` \> \[ `namespace` \> \] `kind` \> \[ `name` \> \] `resources count` \>
    fn get_path(&self, scope: Option<&Scope>) -> Line<'_> {
        let data = &self.app_data.borrow();
        let kind = match self.fixed_kind.as_ref() {
            Some(kind) => kind,
            None => data.current.resource.kind.name(),
        };
        let name = if self.fixed_kind.is_some() {
            None
        } else if let Some(filter) = data.current.resource.filter.as_ref() {
            filter.name.as_deref()
        } else {
            data.current.resource.name.as_deref()
        };

        let mut line = get_left_breadcrumbs(
            data,
            scope,
            self.fixed_namespace.as_deref(),
            kind,
            name,
            self.count,
            self.is_filtered,
        );

        if !self.hide_previous
            && let Some(previous) = self.app_data.borrow().previous.last()
        {
            line.push_span(Span::from(format!(" 󰕍 {}", previous.resource.kind.name())).style(&data.theme.colors.header.previous));
        }

        line
    }

    /// Returns formatted k8s version info as breadcrumbs:\
    /// \< `k8s version` \<
    fn get_version(&mut self) -> Line<'_> {
        let data = &self.app_data.borrow();
        let (text, colors) = get_version_text(data, &mut self.spinner);
        get_right_breadcrumbs(text, colors, data.theme.colors.text.bg)
    }
}

/// Returns kubernetes version text together with its colors.
fn get_version_text<'a>(data: &'a AppData, spinner: &mut Spinner) -> (String, &'a TextColors) {
    let colors;
    let text;

    if data.is_connected {
        colors = &data.theme.colors.header.info;
        text = format!(" {} ", &data.current.version);
    } else {
        colors = &data.theme.colors.header.disconnected;
        if data.current.version.is_empty() {
            text = format!(" {} connecting… ", spinner.tick());
        } else {
            text = format!(" {} {} ", spinner.tick(), &data.current.version);
        }
    }

    (text, colors)
}
