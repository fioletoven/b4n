use b4n_kube::{InitData, ObserverResult, ResourceRef};
use b4n_tui::table::{Table, ViewType};
use b4n_tui::utils::center;
use b4n_tui::widgets::Spinner;
use kube::api::DynamicObject;
use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::text::{Line, Span};
use std::rc::Rc;
use std::time::Instant;

use crate::core::SharedAppData;
use crate::kube::resources::{ResourceItem, ResourcesList};
use crate::ui::presentation::ListViewer;

pub struct DescribeContent {
    app_data: SharedAppData,
    resource: ResourceRef,
    object: Option<DynamicObject>,
    conditions: ListViewer<ResourcesList>,
    events: ListViewer<ResourcesList>,
    creation_time: Instant,
    spinner: Spinner,
}

impl DescribeContent {
    pub fn new(app_data: SharedAppData, resource: ResourceRef) -> Self {
        let conditions = ListViewer::new(Rc::clone(&app_data), ResourcesList::default(), ViewType::Compact);
        let events = ListViewer::new(Rc::clone(&app_data), ResourcesList::default(), ViewType::Compact);

        Self {
            app_data,
            resource,
            object: None,
            conditions,
            events,
            creation_time: Instant::now(),
            spinner: Spinner::default(),
        }
    }

    pub fn update_resource(&mut self, result: ObserverResult<DynamicObject>) {
        match result {
            ObserverResult::Apply(resource) => self.object = Some(resource),
            ObserverResult::Delete(_) => self.object = None,
            _ => (),
        }

        self.rebuild_conditions();
    }

    pub fn update_events(&mut self, result: ObserverResult<ResourceItem>) {
        self.events.table.update(result);
    }

    pub fn draw(&mut self, frame: &mut Frame<'_>, area: Rect) {
        if self.object.is_none() && self.creation_time.elapsed().as_millis() > 200 {
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

    fn rebuild_conditions(&mut self) {
        self.conditions.table.update(ObserverResult::Init(Box::new(InitData::simple(
            self.resource.clone(),
            "Condition".to_owned(),
            "conditions".to_owned(),
        ))));

        let conditions = self.object.as_ref().and_then(|r| r.data["status"]["conditions"].as_array());
        if let Some(conditions) = conditions {
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
