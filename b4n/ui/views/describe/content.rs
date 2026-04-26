use b4n_config::themes::{TextColors, YamlSyntaxColors};
use b4n_kube::{InitData, ObserverResult, ResourceRef};
use b4n_tui::table::{Table, ViewType};
use b4n_tui::utils::center;
use b4n_tui::widgets::Spinner;
use b4n_tui::{MouseEventKind, ResponseEvent, TuiEvent};
use crossterm::event::KeyCode;
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
use crate::kube::resources::{ResourceItem, ResourcesList};
use crate::ui::presentation::{ListViewer, StyledLine, StyledLineExt};

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
    scroll: usize,
    max_height: usize,
    area: Rect,
}

impl DescribeContent {
    /// Creates new [`DescribeContent`] instance.
    pub fn new(app_data: SharedAppData, resource: ResourceRef) -> Self {
        let conditions = ListViewer::new(Rc::clone(&app_data), ResourcesList::default(), ViewType::Compact).with_no_border();
        let events = ListViewer::new(Rc::clone(&app_data), ResourcesList::default(), ViewType::Compact).with_no_border();

        Self {
            app_data,
            resource,
            lines: Vec::new(),
            conditions,
            events,
            creation_time: Instant::now(),
            has_data: false,
            spinner: Spinner::default(),
            scroll: 0,
            max_height: 0,
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
                x if x.code == KeyCode::Home => self.scroll = 0,
                x if x.code == KeyCode::PageUp => {
                    self.scroll = self.scroll.saturating_sub(self.area.height.into());
                },
                x if x.code == KeyCode::Up => self.scroll = self.scroll.saturating_sub(1),
                x if x.code == KeyCode::Down => self.scroll += 1,
                x if x.code == KeyCode::PageDown => self.scroll += usize::from(self.area.height),
                x if x.code == KeyCode::End => self.scroll = self.max_height,

                _ => return ResponseEvent::NotHandled,
            },
            TuiEvent::Mouse(mouse) => match mouse {
                x if x.kind == MouseEventKind::ScrollUp => self.scroll = self.scroll.saturating_sub(1),
                x if x.kind == MouseEventKind::ScrollDown => self.scroll += 1,

                _ => return ResponseEvent::NotHandled,
            },
            TuiEvent::Command(_) => return ResponseEvent::NotHandled,
        }

        self.update_page_start();
        ResponseEvent::Handled
    }

    /// Redraws describe view content on the screen.
    pub fn draw(&mut self, frame: &mut Frame<'_>, area: Rect) {
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
        let sections = vec![
            Section::Lines {
                height: u16::try_from(self.lines.len()).unwrap_or_default(),
            },
            Section::Spacer { height: 1 },
            Section::Conditions {
                height: u16::try_from(self.conditions.table.len()).unwrap_or_default() + 1,
            },
            Section::Spacer { height: 1 },
            Section::Events {
                height: u16::try_from(self.events.table.len()).unwrap_or_default() + 1,
            },
        ];

        self.area = area;
        self.max_height = sections.iter().map(|s| usize::from(s.height())).sum();
        self.draw_sections(frame, area, &sections);
    }

    fn draw_sections(&mut self, frame: &mut Frame<'_>, area: Rect, sections: &[Section]) {
        let scroll = u16::try_from(self.scroll).unwrap_or(u16::MAX);
        let mut current_y = 0u16;
        let viewport_start = scroll;
        let viewport_end = scroll.saturating_add(area.height);

        for section in sections {
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

                    self.draw_section(frame, section, screen_rect, clip_top);
                }
            }

            current_y = section_end;
        }
    }

    fn draw_section(&mut self, frame: &mut Frame<'_>, section: &Section, area: Rect, offset: u16) {
        match section {
            Section::Lines { .. } => {
                frame.render_widget(
                    Paragraph::new(self.get_page_lines(offset.into(), area.height.into())),
                    area.inner(Margin::new(1, 0)),
                );
            },
            Section::Conditions { .. } => {
                self.conditions.draw_clipped(frame, area, offset as usize);
            },
            Section::Events { .. } => {
                self.events.draw_clipped(frame, area, offset as usize);
            },
            Section::Spacer { .. } => {},
        }
    }

    fn update_page_start(&mut self) {
        let max_height = self.max_height.saturating_sub(self.area.height.into());
        if self.scroll > max_height {
            self.scroll = max_height;
        }
    }

    fn get_page_lines(&mut self, start: usize, len: usize) -> Vec<Line<'_>> {
        self.lines.iter().skip(start).take(len).map(|line| line.as_line(0)).collect()
    }

    fn update_describe(&mut self, object: &DynamicObject) {
        let colors = &self.app_data.borrow().theme.colors.syntax.yaml;
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
enum Section {
    Spacer { height: u16 },
    Lines { height: u16 },
    Conditions { height: u16 },
    Events { height: u16 },
}

impl Section {
    fn height(&self) -> u16 {
        match self {
            Section::Spacer { height } => *height,
            Section::Lines { height } => *height,
            Section::Conditions { height } => *height,
            Section::Events { height } => *height,
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
