use b4n_config::themes::{TextColors, YamlSyntaxColors};
use b4n_kube::{InitData, ObserverResult, ResourceRef};
use b4n_tui::table::{Table, ViewType};
use b4n_tui::utils::center;
use b4n_tui::widgets::Spinner;
use b4n_tui::{MouseEventKind, ResponseEvent, TuiEvent};
use crossterm::event::{KeyCode, KeyModifiers};
use kube::ResourceExt;
use kube::api::DynamicObject;
use ratatui::Frame;
use ratatui::layout::{Constraint, Margin, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use std::rc::Rc;
use std::time::Instant;

use crate::core::SharedAppData;
use crate::kube::resources::{ColumnsLayout, ResourceItem, ResourcesList};
use crate::ui::presentation::{ContentPosition, ListViewer, StyledLine, StyledLineExt};

/// Describe resource content.
pub struct DescribeContent {
    app_data: SharedAppData,
    resource: ResourceRef,
    lines: Vec<StyledLine>,
    conditions: ListViewer<ResourcesList>,
    events: ListViewer<ResourcesList>,
    creation_time: Instant,
    has_data: bool,
    spinner: Spinner,
    page_start: ContentPosition,
    max_height: usize,
    max_width: usize,
    area: Rect,
}

impl DescribeContent {
    /// Creates new [`DescribeContent`] instance.
    pub fn new(app_data: SharedAppData, resource: ResourceRef) -> Self {
        let mut conditions = ListViewer::new(
            Rc::clone(&app_data),
            ResourcesList::default().with_focus(false),
            ViewType::Compact,
        )
        .with_no_border()
        .with_focus(false);
        let mut events = ListViewer::new(
            Rc::clone(&app_data),
            ResourcesList::default()
                .with_columns_layout(ColumnsLayout::Compact)
                .with_focus(false),
            ViewType::Compact,
        )
        .with_no_border()
        .with_focus(false);

        conditions.table.table.limit_offset(false);
        events.table.table.limit_offset(false);

        Self {
            app_data,
            resource,
            lines: Vec::new(),
            conditions,
            events,
            creation_time: Instant::now(),
            has_data: false,
            spinner: Spinner::default(),
            page_start: ContentPosition::new(0, 0),
            max_height: 0,
            max_width: 0,
            area: Rect::default(),
        }
    }

    /// Updates resource that is currently described.
    pub fn update_resource(&mut self, result: ObserverResult<DynamicObject>) {
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

    /// Returns current page coordinates.
    pub fn get_coordinates(&self) -> ContentPosition {
        self.page_start
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
            Section::Spacer(1, 1),
            Section::from_list(&mut self.conditions),
            Section::Spacer(1, 1),
            Section::from_list(&mut self.events),
        ];

        self.max_height = sections.iter().map(|s| usize::from(s.height())).sum();
        self.max_width = sections.iter().map(Section::width).max().unwrap_or_default();
        Self::draw_sections(frame, area, &mut sections, self.page_start);
    }

    fn draw_sections(frame: &mut Frame<'_>, area: Rect, sections: &mut [Section], page_start: ContentPosition) {
        let scroll_y = u16::try_from(page_start.y).unwrap_or(u16::MAX);
        let mut current_y = 0u16;
        let viewport_start = scroll_y;
        let viewport_end = scroll_y.saturating_add(area.height);

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

                    section.draw(frame, screen_rect, clip_top, page_start.x);
                }
            }

            current_y = section_end;
        }
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

    fn update_describe(&mut self, object: &DynamicObject) {
        let colors = &self.app_data.borrow().theme.colors.syntax.describe;
        self.lines.clear();
        self.lines.push(property(colors, "name", object.name_any()));
        if let Some(namespace) = object.metadata.namespace.as_deref() {
            self.lines.push(property(colors, "namespace", namespace));
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
}

/// Represents a section in the describe view.
enum Section<'a> {
    Spacer(usize, u16),
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
            Section::Spacer(width, _) | Section::Text(_, width, _) | Section::List(_, width, _) => *width,
        }
    }

    fn height(&self) -> u16 {
        match self {
            Section::Spacer(_, height) | Section::Text(_, _, height) | Section::List(_, _, height) => *height,
        }
    }

    fn draw(&mut self, frame: &mut Frame<'_>, area: Rect, offset_y: u16, offset_x: usize) {
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
                list.table.table.set_offset(offset_x);
                list.draw_clipped(frame, area, offset_y as usize);
            },
            Section::Spacer(_, _) => {},
        }
    }
}

fn span(color: &TextColors, text: impl Into<String>) -> (Style, String) {
    (color.into(), text.into())
}

fn property(colors: &YamlSyntaxColors, name: impl Into<String>, value: impl Into<String>) -> StyledLine {
    vec![
        span(&colors.property, name),
        span(&colors.normal, ": "),
        span(&colors.string, value),
    ]
}
