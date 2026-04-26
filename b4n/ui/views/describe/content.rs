use b4n_config::themes::{TextColors, YamlSyntaxColors};
use b4n_kube::{InitData, ObserverResult, ResourceRef};
use b4n_tui::table::{Table, ViewType};
use b4n_tui::utils::center;
use b4n_tui::widgets::Spinner;
use kube::ResourceExt;
use kube::api::DynamicObject;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Margin, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use std::rc::Rc;
use std::time::Instant;

use crate::core::SharedAppData;
use crate::kube::resources::{ResourceItem, ResourcesList};
use crate::ui::presentation::{ListViewer, StyledLine, StyledLineExt};

pub struct DescribeContent {
    app_data: SharedAppData,
    resource: ResourceRef,
    lines: Vec<StyledLine>,
    conditions: ListViewer<ResourcesList>,
    events: ListViewer<ResourcesList>,
    creation_time: Instant,
    has_data: bool,
    spinner: Spinner,
}

impl DescribeContent {
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
        }
    }

    pub fn update_resource(&mut self, result: ObserverResult<DynamicObject>) {
        let (ObserverResult::Apply(object) | ObserverResult::Delete(object)) = result else {
            return;
        };

        self.has_data = true;
        self.update_describe(&object);
        self.update_conditions(&object);
    }

    pub fn update_events(&mut self, result: ObserverResult<ResourceItem>) {
        self.events.table.update(result);
    }

    pub fn draw(&mut self, frame: &mut Frame<'_>, area: Rect) {
        if self.has_data {
            self.draw_content(frame, area);
        } else if self.creation_time.elapsed().as_millis() > 200 {
            self.draw_empty(frame, area);
        }
    }

    fn draw_content(&mut self, frame: &mut Frame<'_>, area: Rect) {
        let lines_height = u16::try_from(self.lines.len()).unwrap_or_default();
        let conditions_height = u16::try_from(self.conditions.table.len()).unwrap_or_default() + 1;
        let events_height = u16::try_from(self.events.table.len()).unwrap_or_default() + 1;
        let virtual_area = Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: lines_height + 1 + conditions_height + 1 + events_height,
        };
        let layout = Layout::default()
            .constraints([
                Constraint::Length(lines_height),
                Constraint::Length(1),
                Constraint::Length(conditions_height),
                Constraint::Length(1),
                Constraint::Length(events_height),
            ])
            .split(virtual_area);

        frame.render_widget(
            Paragraph::new(self.get_page_lines(0, layout[0].height.into())),
            layout[0].inner(Margin::new(1, 0)),
        );
        self.conditions.draw(frame, layout[2]);
        self.events.draw(frame, layout[4]);
    }

    fn draw_empty(&mut self, frame: &mut Frame<'_>, area: Rect) {
        let colors = &self.app_data.borrow().theme.colors;
        let line = Line::default()
            .spans([Span::raw(self.spinner.tick().to_string()), " waiting for data…".into()])
            .style(&colors.text);
        let area = center(area, Constraint::Length(line.width() as u16), Constraint::Length(4));
        frame.render_widget(line, area);
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
