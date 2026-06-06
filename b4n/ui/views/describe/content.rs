use b4n_config::keys::KeyCommand;
use b4n_config::themes::YamlSyntaxColors;
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
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::time::Instant;

use crate::core::{SharedAppData, SharedAppDataExt};
use crate::kube::resources::{ColumnsLayout, ResourceItem, ResourcesList};
use crate::ui::presentation::{ContentPosition, ListViewer, StyledLine};
use crate::ui::views::describe::data::{self, SectionData};
use crate::ui::views::describe::utils::{ValueKind, header, list, none, property};
use crate::ui::widgets::table::BasicTable;

/// Describe resource content.
pub struct DescribeContent {
    app_data: SharedAppData,
    resource: ResourceRef,
    lines_start: Vec<StyledLine>,
    lines_end: Vec<StyledLine>,
    conditions: ListViewer<ResourcesList>,
    conditions_header: Vec<StyledLine>,
    events: ListViewer<ResourcesList>,
    events_header: Vec<StyledLine>,
    sections: Vec<SectionData>,
    creation_time: Instant,
    has_data: bool,
    is_deleted: bool,
    spinner: Spinner,
    page_start: ContentPosition,
    max_height: usize,
    max_width: usize,
    section_targets: Vec<(Rect, Rect, Option<FocusTarget>)>,
    focused: FocusTarget,
    area: Rect,
}

impl DescribeContent {
    /// Creates new [`DescribeContent`] instance.
    pub fn new(app_data: SharedAppData, resource: ResourceRef) -> Self {
        let (conditions, conditions_header) = Self::create_conditions(&app_data);
        let (events, events_header) = Self::create_events(&app_data);
        let sections = data::create_additional_sections(&resource, &app_data);

        Self {
            app_data,
            resource,
            lines_start: Vec::new(),
            lines_end: Vec::new(),
            conditions,
            conditions_header,
            events,
            events_header,
            sections,
            creation_time: Instant::now(),
            has_data: false,
            is_deleted: false,
            spinner: Spinner::default(),
            page_start: ContentPosition::new(0, 0),
            max_height: 0,
            max_width: 0,
            section_targets: Vec::new(),
            focused: FocusTarget::Scroll,
            area: Rect::default(),
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
        self.update_additional_sections(&object);
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
            FocusTarget::Scroll => self.process_scroll_event(event),
            FocusTarget::AdditionalSection(index) => match self.sections.get_mut(index) {
                Some(SectionData::Resources(list)) => list.process_event(event),
                Some(SectionData::List(list)) => list.process_event(event),
                _ => ResponseEvent::NotHandled,
            },
            FocusTarget::Conditions => self.conditions.process_event(event),
            FocusTarget::Events => self.events.process_event(event),
        }
    }

    /// Returns focused list as a text.
    pub fn get_focused_list_text(&mut self) -> Option<String> {
        let lines = match self.focused {
            FocusTarget::Scroll => Vec::new(),
            FocusTarget::AdditionalSection(index) => match self.sections.get_mut(index) {
                Some(SectionData::Resources(list)) => list.table.get_items_as_text(ViewType::Compact, false),
                Some(SectionData::List(list)) => list.table.get_items_as_text(ViewType::Compact, false),
                _ => Vec::new(),
            },
            FocusTarget::Conditions => self.conditions.table.get_items_as_text(ViewType::Compact, false),
            FocusTarget::Events => self.events.table.get_items_as_text(ViewType::Compact, false),
        };
        if lines.is_empty() { None } else { Some(lines.join("\n")) }
    }

    /// Returns `true` if content can be scrolled.
    pub fn is_in_scroll_mode(&self) -> bool {
        self.focused == FocusTarget::Scroll
    }

    /// Returns current page coordinates.\
    /// **Note** that it returns them only if page scrolling is possible.
    pub fn get_coordinates(&self) -> Option<ContentPosition> {
        if self.focused == FocusTarget::Scroll {
            Some(self.page_start)
        } else {
            None
        }
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
        let header = vec![StyledLine::default(), header(colors, "Conditions", 0)];

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
        let header = vec![StyledLine::default(), header(colors, "Events", 0)];

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
        let mut sections = Vec::with_capacity(self.sections.len() + 6);
        let mut section_targets = Vec::with_capacity(self.sections.len() + 6);

        sections.push(Section::from_text(&mut self.lines_start));
        section_targets.push(None);

        for (index, section) in self.sections.iter_mut().enumerate() {
            match section {
                SectionData::Text(lines) => {
                    sections.push(Section::from_text(lines));
                    section_targets.push(None);
                },
                SectionData::Resources(list) => {
                    sections.push(Section::from_resources_list(list));
                    section_targets.push(Some(FocusTarget::AdditionalSection(index)));
                },
                SectionData::List(list) => {
                    sections.push(Section::from_basic_list(list));
                    section_targets.push(Some(FocusTarget::AdditionalSection(index)));
                },
            }
        }

        sections.push(Section::from_text(&mut self.lines_end));
        section_targets.push(None);

        sections.push(Section::from_text(&mut self.conditions_header));
        section_targets.push(None);
        sections.push(Section::from_resources_list(&mut self.conditions));
        section_targets.push(Some(FocusTarget::Conditions));

        sections.push(Section::from_text(&mut self.events_header));
        section_targets.push(None);
        sections.push(Section::from_resources_list(&mut self.events));
        section_targets.push(Some(FocusTarget::Events));

        self.max_height = sections.iter().map(|s| usize::from(s.height())).sum();
        self.max_width = sections.iter().map(Section::width).max().unwrap_or_default();
        self.section_targets = Self::draw_sections(frame, area, &self.app_data, &mut sections, &section_targets, self.page_start);
    }

    fn draw_sections(
        frame: &mut Frame<'_>,
        area: Rect,
        app_data: &SharedAppData,
        sections: &mut [Section],
        section_targets: &[Option<FocusTarget>],
        page_start: ContentPosition,
    ) -> Vec<(Rect, Rect, Option<FocusTarget>)> {
        let scroll_y = u16::try_from(page_start.y).unwrap_or(u16::MAX);
        let mut current_y = 0u16;
        let viewport_start = scroll_y;
        let viewport_end = scroll_y.saturating_add(area.height);
        let mut areas = Vec::new();

        for (section, target) in sections.iter_mut().zip(section_targets.iter().copied()) {
            let section_height = section.height();
            let section_start = current_y;
            let section_end = current_y.saturating_add(section_height);
            let page_rect = Rect {
                x: 0,
                y: section_start,
                width: u16::try_from(section.width()).unwrap_or(u16::MAX),
                height: section_height,
            };

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
                    areas.push((screen_rect, page_rect, target));
                } else {
                    areas.push((Rect::default(), page_rect, target));
                }
            } else {
                areas.push((Rect::default(), page_rect, target));
            }

            current_y = section_end;
        }

        areas
    }

    fn get_clicked_section(&self, event: &TuiEvent) -> FocusTarget {
        for (area, _, target) in &self.section_targets {
            if let Some(target) = target
                && event.is_in(MouseEventKind::LeftClick, *area)
            {
                return *target;
            }
        }

        FocusTarget::Scroll
    }

    fn can_focus_section(&self, section: FocusTarget) -> bool {
        match section {
            FocusTarget::Scroll => true,
            FocusTarget::AdditionalSection(index) => match self.sections.get(index) {
                Some(SectionData::Resources(list)) => !list.table.is_empty(),
                Some(SectionData::List(list)) => !list.table.is_empty(),
                _ => false,
            },
            FocusTarget::Conditions => !self.conditions.table.is_empty(),
            FocusTarget::Events => !self.events.table.is_empty(),
        }
    }

    fn focus_section(&mut self, section: FocusTarget) {
        if self.focused != section {
            self.focused = if self.can_focus_section(section) {
                section
            } else {
                FocusTarget::Scroll
            };
            self.update_list_focuses();
        }
    }

    fn focus_next_section(&mut self) {
        let targets = self.focus_targets();
        if let Some(index) = targets.iter().position(|(target, _)| *target == self.focused) {
            let (target, area) = targets[(index + 1) % targets.len()];
            self.focus_section(target);
            if target != FocusTarget::Scroll {
                self.ensure_section_visible(area);
            }
        }
    }

    fn ensure_section_visible(&mut self, section: Rect) {
        let page_start = self.page_start.y;
        let section_start = usize::from(section.y);

        if section_start < page_start {
            self.page_start.x = 0;
            self.page_start.y = section_start;
            self.update_page_start();

            return;
        }

        let page_end = page_start.saturating_add(self.area.height.into());
        let section_end = usize::from(section.y.saturating_add(section.height));

        if section_end > page_end {
            self.page_start.x = 0;
            self.page_start.y = section_end.saturating_sub(self.area.height.into());
            self.update_page_start();
        }
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
        self.lines_start.clear();
        self.lines_end.clear();

        self.lines_start
            .push(property(colors, "Name", object.name_any(), ValueKind::String, 0));
        if let Some(namespace) = object.metadata.namespace.as_deref() {
            self.lines_start
                .push(property(colors, "Namespace", namespace, ValueKind::String, 0));
        }

        add_list(&mut self.lines_start, colors, "Labels", object.metadata.labels.as_ref());
        add_list(
            &mut self.lines_start,
            colors,
            "Annotations",
            object.metadata.annotations.as_ref(),
        );

        self.lines_end.push(StyledLine::default());
        self.lines_end
            .push(property(colors, "Status", status::from_object(object), ValueKind::String, 0));
    }

    fn update_additional_sections(&mut self, object: &DynamicObject) {
        data::update_additional_sections(&self.resource, &self.app_data, object, &mut self.sections);
    }

    fn update_list_focuses(&mut self) {
        self.conditions.set_focus(self.focused == FocusTarget::Conditions);
        self.events.set_focus(self.focused == FocusTarget::Events);
        for (index, section) in self.sections.iter_mut().enumerate() {
            match section {
                SectionData::Resources(list) => list.set_focus(self.focused == FocusTarget::AdditionalSection(index)),
                SectionData::List(list) => list.set_focus(self.focused == FocusTarget::AdditionalSection(index)),
                SectionData::Text(_) => (),
            }
        }
    }

    fn focus_targets(&self) -> Vec<(FocusTarget, Rect)> {
        let mut targets = vec![(FocusTarget::Scroll, Rect::default())];

        for (_, area, target) in &self.section_targets {
            if let Some(target) = target
                && self.can_focus_section(*target)
            {
                targets.push((*target, *area));
            }
        }

        targets
    }
}

/// Represents focus target in the describe view.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FocusTarget {
    Scroll,
    AdditionalSection(usize),
    Conditions,
    Events,
}

/// Represents a section in the describe view.
enum Section<'a> {
    Text(&'a mut Vec<StyledLine>, usize, u16),
    Resources(&'a mut ListViewer<ResourcesList>, usize, u16),
    List(&'a mut ListViewer<BasicTable>, usize, u16),
}

impl<'a> Section<'a> {
    fn from_text(value: &'a mut Vec<StyledLine>) -> Self {
        let width = value.iter().map(StyledLine::chars_len).max();
        let height = u16::try_from(value.len()).unwrap_or_default();
        Section::Text(value, width.unwrap_or_default(), height)
    }

    fn from_resources_list(value: &'a mut ListViewer<ResourcesList>) -> Self {
        let width = value.table.table.header.get_cached_length().unwrap_or_default();
        let height = u16::try_from(value.table.len()).unwrap_or_default() + 1;
        Self::Resources(value, width, height)
    }

    fn from_basic_list(value: &'a mut ListViewer<BasicTable>) -> Self {
        let width = value.table.table.header.get_cached_length().unwrap_or_default();
        let height = u16::try_from(value.table.len()).unwrap_or_default() + 1;
        Self::List(value, width, height)
    }

    fn width(&self) -> usize {
        match self {
            Section::Text(_, width, _) | Section::Resources(_, width, _) | Section::List(_, width, _) => *width,
        }
    }

    fn height(&self) -> u16 {
        match self {
            Section::Text(_, _, height) | Section::Resources(_, _, height) | Section::List(_, _, height) => *height,
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
            Section::Resources(list, _, _) => {
                if list.table.is_empty() {
                    let colors = &app_data.borrow().theme.colors.syntax.describe;
                    frame.render_widget(Paragraph::new(none(colors).as_line(offset_x)), area.inner(Margin::new(1, 0)));
                } else {
                    list.table.table.set_offset(offset_x);
                    list.draw_clipped(frame, area, offset_y as usize);
                }
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

fn add_list(lines: &mut Vec<StyledLine>, colors: &YamlSyntaxColors, title: &str, source: Option<&BTreeMap<String, String>>) {
    lines.push(StyledLine::default());
    lines.push(header(colors, title, 0));

    if let Some(source) = source {
        lines.append(&mut list(colors, source));
    } else {
        lines.push(none(colors));
    }
}
