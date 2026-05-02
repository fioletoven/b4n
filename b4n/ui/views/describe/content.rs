use b4n_config::keys::KeyCommand;
use b4n_config::themes::{TextColors, YamlSyntaxColors};
use b4n_kube::{InitData, ObserverResult, ResourceRef, status};
use b4n_tui::table::{Table, ViewType};
use b4n_tui::utils::center;
use b4n_tui::widgets::Spinner;
use b4n_tui::{MouseEventKind, ResponseEvent, Responsive, TuiEvent};
use crossterm::event::{KeyCode, KeyModifiers};
use kube::ResourceExt;
use kube::api::DynamicObject;
use ratatui::Frame;
use ratatui::layout::{Constraint, Margin, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::time::Instant;

use crate::core::{SharedAppData, SharedAppDataExt};
use crate::kube::resources::{ColumnsLayout, ResourceItem, ResourcesList};
use crate::ui::presentation::{ContentPosition, ListViewer, StyledLine, StyledLineExt};

/// Describe resource content.
pub struct DescribeContent {
    app_data: SharedAppData,
    resource: ResourceRef,
    lines: Vec<StyledLine>,
    conditions: ListViewer<ResourcesList>,
    conditions_header: Vec<StyledLine>,
    events: ListViewer<ResourcesList>,
    events_header: Vec<StyledLine>,
    creation_time: Instant,
    has_data: bool,
    is_deleted: bool,
    spinner: Spinner,
    page_start: ContentPosition,
    max_height: usize,
    max_width: usize,
    area: Rect,
    section_areas: Vec<Rect>,
    focused: u8,
}

impl DescribeContent {
    /// Creates new [`DescribeContent`] instance.
    pub fn new(app_data: SharedAppData, resource: ResourceRef) -> Self {
        let (conditions, conditions_header) = Self::create_conditions(&app_data);
        let (events, events_header) = Self::create_events(&app_data);
        Self {
            app_data,
            resource,
            lines: Vec::new(),
            conditions,
            conditions_header,
            events,
            events_header,
            creation_time: Instant::now(),
            has_data: false,
            is_deleted: false,
            spinner: Spinner::default(),
            page_start: ContentPosition::new(0, 0),
            max_height: 0,
            max_width: 0,
            area: Rect::default(),
            section_areas: Vec::new(),
            focused: 0,
        }
    }

    /// Updates resource that is currently described.
    pub fn update_resource(&mut self, result: ObserverResult<DynamicObject>) {
        if self.is_deleted {
            return;
        }

        self.is_deleted = matches!(result, ObserverResult::Delete(_));
        let (ObserverResult::Apply(object) | ObserverResult::Delete(object)) = result else {
            return;
        };

        self.has_data = true;
        self.update_describe(&object);
        self.update_conditions(&object);
    }

    /// Updates described resource events.
    pub fn update_events(&mut self, result: ObserverResult<ResourceItem>) {
        self.events.table.update(result);
    }

    /// Processes UI key/mouse event.
    pub fn process_event(&mut self, event: &TuiEvent) -> ResponseEvent {
        if self.app_data.has_binding(event, KeyCommand::NavigateNext) {
            self.focus_next_section();
            return ResponseEvent::Handled;
        }

        if event.is_mouse(MouseEventKind::LeftClick) {
            self.focus_section(self.get_clicked_section(event));
        }

        match self.focused {
            1 => self.conditions.process_event(event),
            2 => self.events.process_event(event),
            _ => self.process_scroll_event(event),
        }
    }

    /// Returns current page coordinates.\
    /// **Note** that it returns them only if page scrolling is possible.
    pub fn get_coordinates(&self) -> Option<ContentPosition> {
        if self.focused == 0 { Some(self.page_start) } else { None }
    }

    /// Redraws describe view content on the screen.
    pub fn draw(&mut self, frame: &mut Frame<'_>, area: Rect) {
        if self.area != area {
            self.area = area;
            self.update_page_start();
        }

        if self.has_data {
            self.draw_content(frame, area);
        } else if self.creation_time.elapsed().as_millis() > 200 {
            self.draw_empty(frame, area);
        }
    }

    fn create_conditions(app_data: &SharedAppData) -> (ListViewer<ResourcesList>, Vec<StyledLine>) {
        let mut viewer = ListViewer::new(
            Rc::clone(app_data),
            ResourcesList::default().with_focus(false),
            ViewType::Compact,
        )
        .with_no_border()
        .with_focus(false);
        viewer.table.table.limit_offset(false);

        let colors = &app_data.borrow().theme.colors.syntax.describe;
        let header = vec![StyledLine::default(), property(colors, "Conditions", "")];

        (viewer, header)
    }

    fn create_events(app_data: &SharedAppData) -> (ListViewer<ResourcesList>, Vec<StyledLine>) {
        let mut viewer = ListViewer::new(
            Rc::clone(app_data),
            ResourcesList::default()
                .with_columns_layout(ColumnsLayout::Compact)
                .with_focus(false),
            ViewType::Compact,
        )
        .with_no_border()
        .with_focus(false);
        viewer.table.table.limit_offset(false);

        let colors = &app_data.borrow().theme.colors.syntax.describe;
        let header = vec![StyledLine::default(), property(colors, "Events", "")];

        (viewer, header)
    }

    fn draw_empty(&mut self, frame: &mut Frame<'_>, area: Rect) {
        let colors = &self.app_data.borrow().theme.colors;
        let line = Line::default()
            .spans([Span::raw(self.spinner.tick().to_string()), " waiting for data…".into()])
            .style(&colors.text);
        let area = center(area, Constraint::Length(line.width() as u16), Constraint::Length(4));
        frame.render_widget(line, area);
    }

    fn draw_content(&mut self, frame: &mut Frame<'_>, area: Rect) {
        let mut sections = vec![
            Section::from_text(&mut self.lines),
            Section::from_text(&mut self.conditions_header),
            Section::from_list(&mut self.conditions),
            Section::from_text(&mut self.events_header),
            Section::from_list(&mut self.events),
        ];

        self.max_height = sections.iter().map(|s| usize::from(s.height())).sum();
        self.max_width = sections.iter().map(Section::width).max().unwrap_or_default();
        self.section_areas = Self::draw_sections(frame, area, &self.app_data, &mut sections, self.page_start);
    }

    fn draw_sections(
        frame: &mut Frame<'_>,
        area: Rect,
        app_data: &SharedAppData,
        sections: &mut [Section],
        page_start: ContentPosition,
    ) -> Vec<Rect> {
        let scroll_y = u16::try_from(page_start.y).unwrap_or(u16::MAX);
        let mut current_y = 0u16;
        let viewport_start = scroll_y;
        let viewport_end = scroll_y.saturating_add(area.height);
        let mut areas = Vec::new();

        for section in sections.iter_mut() {
            let section_height = section.height();
            let section_start = current_y;
            let section_end = current_y.saturating_add(section_height);

            if section_end > viewport_start && section_start < viewport_end {
                let clip_top = viewport_start.saturating_sub(section_start);
                let clip_bottom = section_end.saturating_sub(viewport_end);
                let visible_height = section_height.saturating_sub(clip_top).saturating_sub(clip_bottom);

                if visible_height > 0 {
                    let screen_y = section_start.saturating_sub(viewport_start);
                    let screen_rect = Rect {
                        x: area.x,
                        y: area.y.saturating_add(screen_y),
                        width: area.width,
                        height: visible_height.min(area.height.saturating_sub(screen_y)),
                    };

                    section.draw(frame, screen_rect, app_data, clip_top, page_start.x);
                    areas.push(screen_rect);
                } else {
                    areas.push(Rect::default());
                }
            } else {
                areas.push(Rect::default());
            }

            current_y = section_end;
        }

        areas
    }

    fn get_clicked_section(&self, event: &TuiEvent) -> u8 {
        if self.section_areas.len() > 4 {
            if event.is_in(MouseEventKind::LeftClick, self.section_areas[2]) {
                return 1;
            }

            if event.is_in(MouseEventKind::LeftClick, self.section_areas[4]) {
                return 2;
            }
        }

        0
    }

    fn can_focus_section(&self, section: u8) -> bool {
        match section {
            1 => !self.conditions.table.is_empty(),
            2 => !self.events.table.is_empty(),
            _ => false,
        }
    }

    fn focus_section(&mut self, section: u8) {
        if self.focused != section {
            self.focused = if self.can_focus_section(section) { section } else { 0 };
            self.conditions.set_focus(self.focused == 1);
            self.events.set_focus(self.focused == 2);
        }
    }

    fn focus_next_section(&mut self) {
        let section = if self.focused == 2 { 0 } else { self.focused + 1 };
        self.focus_section(section);
    }

    fn process_scroll_event(&mut self, event: &TuiEvent) -> ResponseEvent {
        match event {
            TuiEvent::Key(key) => match key {
                // horizontal scroll
                x if x.code == KeyCode::Home && x.modifiers == KeyModifiers::CONTROL => self.page_start.x = 0,
                x if x.code == KeyCode::PageUp && x.modifiers == KeyModifiers::CONTROL => {
                    self.page_start.sub_x(self.area.width.into());
                },
                x if x.code == KeyCode::Left => self.page_start.sub_x(1),
                x if x.code == KeyCode::Right => self.page_start.add_x(1),
                x if x.code == KeyCode::PageDown && x.modifiers == KeyModifiers::CONTROL => {
                    self.page_start.add_x(usize::from(self.area.width));
                },
                x if x.code == KeyCode::End && x.modifiers == KeyModifiers::CONTROL => self.page_start.x = self.max_width,

                // vertical scroll
                x if x.code == KeyCode::Home => self.page_start.y = 0,
                x if x.code == KeyCode::PageUp => self.page_start.sub_y(self.area.height.into()),
                x if x.code == KeyCode::Up => self.page_start.sub_y(1),
                x if x.code == KeyCode::Down => self.page_start.add_y(1),
                x if x.code == KeyCode::PageDown => self.page_start.add_y(self.area.height.into()),
                x if x.code == KeyCode::End => self.page_start.y = self.max_height,

                _ => return ResponseEvent::NotHandled,
            },
            TuiEvent::Mouse(mouse) => match mouse {
                // horizontal scroll
                x if x.kind == MouseEventKind::ScrollUp && x.modifiers == KeyModifiers::CONTROL => {
                    self.page_start.sub_x(1);
                },
                x if x.kind == MouseEventKind::ScrollDown && x.modifiers == KeyModifiers::CONTROL => self.page_start.add_x(1),
                x if x.kind == MouseEventKind::ScrollLeft => self.page_start.sub_x(1),
                x if x.kind == MouseEventKind::ScrollRight => self.page_start.add_x(1),

                // vertical scroll
                x if x.kind == MouseEventKind::ScrollUp => self.page_start.sub_y(1),
                x if x.kind == MouseEventKind::ScrollDown => self.page_start.add_y(1),

                _ => return ResponseEvent::NotHandled,
            },
            TuiEvent::Command(_) => return ResponseEvent::NotHandled,
        }

        self.update_page_start();
        ResponseEvent::Handled
    }

    fn update_page_start(&mut self) {
        let max_width = self.max_width.saturating_sub(self.area.width.saturating_sub(2).into());
        if self.page_start.x > max_width {
            self.page_start.x = max_width;
        }

        let max_height = self.max_height.saturating_sub(self.area.height.into());
        if self.page_start.y > max_height {
            self.page_start.y = max_height;
        }
    }

    fn update_conditions(&mut self, object: &DynamicObject) {
        self.conditions.table.update(ObserverResult::Init(Box::new(InitData::simple(
            self.resource.clone(),
            "Condition".to_owned(),
            "conditions".to_owned(),
        ))));

        if let Some(conditions) = object.data["status"]["conditions"].as_array() {
            for condition in conditions {
                self.conditions
                    .table
                    .update(ObserverResult::new(ResourceItem::from_status_condition(condition), false));
            }
        }

        self.conditions.table.update(ObserverResult::InitDone);
        self.conditions.table.sort(5, false);
    }

    fn update_describe(&mut self, object: &DynamicObject) {
        let colors = &self.app_data.borrow().theme.colors.syntax.describe;
        self.lines.clear();

        self.lines.push(property(colors, "Name", object.name_any()));
        if let Some(namespace) = object.metadata.namespace.as_deref() {
            self.lines.push(property(colors, "Namespace", namespace));
        }

        add_describe_list(&mut self.lines, colors, "Labels", object.metadata.labels.as_ref());
        add_describe_list(&mut self.lines, colors, "Annotations", object.metadata.annotations.as_ref());

        self.lines.push(StyledLine::default());
        self.lines
            .push(property(colors, "Overall status", status::from_object(object)))
    }
}

/// Represents a section in the describe view.
enum Section<'a> {
    Text(&'a mut Vec<StyledLine>, usize, u16),
    List(&'a mut ListViewer<ResourcesList>, usize, u16),
}

impl<'a> Section<'a> {
    fn from_text(value: &'a mut Vec<StyledLine>) -> Self {
        let width = value
            .iter()
            .map(|l| l.iter().map(|(_, s)| s.chars().count()).sum::<usize>())
            .max();
        let height = u16::try_from(value.len()).unwrap_or_default();
        Section::Text(value, width.unwrap_or_default(), height)
    }

    fn from_list(value: &'a mut ListViewer<ResourcesList>) -> Self {
        let width = value.table.table.header.get_cached_length().unwrap_or_default();
        let height = u16::try_from(value.table.len()).unwrap_or_default() + 1;
        Self::List(value, width, height)
    }

    fn width(&self) -> usize {
        match self {
            Section::Text(_, width, _) | Section::List(_, width, _) => *width,
        }
    }

    fn height(&self) -> u16 {
        match self {
            Section::Text(_, _, height) | Section::List(_, _, height) => *height,
        }
    }

    fn draw(&mut self, frame: &mut Frame<'_>, area: Rect, app_data: &SharedAppData, offset_y: u16, offset_x: usize) {
        match self {
            Section::Text(lines, _, _) => {
                let lines: Vec<Line<'_>> = lines
                    .iter()
                    .skip(offset_y.into())
                    .take(area.height.into())
                    .map(|line| line.as_line(offset_x))
                    .collect();
                frame.render_widget(Paragraph::new(lines), area.inner(Margin::new(1, 0)));
            },
            Section::List(list, _, _) => {
                if list.table.is_empty() {
                    let colors = &app_data.borrow().theme.colors.syntax.describe;
                    frame.render_widget(Paragraph::new(none(colors).as_line(offset_x)), area.inner(Margin::new(1, 0)));
                } else {
                    list.table.table.set_offset(offset_x);
                    list.draw_clipped(frame, area, offset_y as usize);
                }
            },
        }
    }
}

fn span(color: &TextColors, text: impl Into<String>) -> (Style, String) {
    (color.into(), text.into())
}

fn none(colors: &YamlSyntaxColors) -> StyledLine {
    vec![span(&colors.normal, "  --none--")]
}

fn property(colors: &YamlSyntaxColors, name: impl Into<String>, value: impl Into<String>) -> StyledLine {
    vec![
        span(&colors.property, name),
        span(&colors.normal, ": "),
        span(&colors.string, value),
    ]
}

fn element(colors: &YamlSyntaxColors, key: impl Into<String>, value: impl Into<String>) -> StyledLine {
    vec![
        span(&colors.normal, "  - "),
        span(&colors.string, key),
        span(&colors.normal, "="),
        span(&colors.string, value),
    ]
}

fn add_describe_list(
    lines: &mut Vec<StyledLine>,
    colors: &YamlSyntaxColors,
    title: &str,
    list: Option<&BTreeMap<String, String>>,
) {
    lines.push(StyledLine::default());
    lines.push(property(colors, title, ""));

    let mut has_entries = false;
    if let Some(list) = list {
        for (key, value) in list {
            if key != "kubectl.kubernetes.io/last-applied-configuration" {
                has_entries = true;
                lines.push(element(colors, key, value));
            }
        }
    }

    if !has_entries {
        lines.push(none(colors))
    }
}
