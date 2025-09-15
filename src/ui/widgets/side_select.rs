use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    symbols::border,
    widgets::{Block, Borders, Clear, Paragraph},
};
use std::time::Instant;

use crate::{
    core::{SharedAppData, SharedAppDataExt},
    ui::{KeyCommand, ResponseEvent, Responsive, Table, TuiEvent},
};

use super::Select;

/// Possible positions for the [`SideSelect`] widget.
#[derive(PartialEq)]
pub enum Position {
    Left,
    Right,
}

/// Side select widget for TUI.\
/// It can be displayed on the left or right side of the specified area.
pub struct SideSelect<T: Table> {
    pub is_visible: bool,
    pub select: Select<T>,
    app_data: SharedAppData,
    header: String,
    position: Position,
    result: fn(String) -> ResponseEvent,
    width: u16,
    is_key_pressed: bool,
    showup_time: Instant,
}

impl<T: Table> SideSelect<T> {
    /// Creates new [`SideSelect`] instance.
    pub fn new(
        name: &str,
        app_data: SharedAppData,
        list: T,
        position: Position,
        result: fn(String) -> ResponseEvent,
        width: u16,
    ) -> Self {
        let header = format!(" SELECT {name}: ");
        let select = Select::new(list, app_data.borrow().theme.colors.side_select.clone(), true, false);

        SideSelect {
            is_visible: false,
            select,
            app_data,
            header,
            position,
            result,
            width: std::cmp::max(width, 5),
            is_key_pressed: false,
            showup_time: Instant::now(),
        }
    }

    /// Marks [`SideSelect`] as visible, after that it can be drawn on the terminal frame.
    pub fn show(&mut self) {
        self.is_key_pressed = false;
        self.is_visible = true;
        self.select.reset();
        self.select
            .set_colors(self.app_data.borrow().theme.colors.side_select.clone());
        self.showup_time = Instant::now();
    }

    /// Marks [`SideSelect`] as visible and highlights an item by name and group.
    pub fn show_selected(&mut self, selected_name: &str, selected_group: &str) {
        self.select.highlight(selected_name, selected_group);
        self.show();
    }

    /// Marks [`SideSelect`] as hidden.
    pub fn hide(&mut self) {
        self.is_visible = false;
    }

    /// Draws [`SideSelect`] on the provided frame area.
    pub fn draw(&mut self, frame: &mut ratatui::Frame<'_>, area: Rect) {
        if !self.is_visible {
            return;
        }

        let area = self.get_positioned_area(area);
        let block = self.get_positioned_block();
        let inner_area = block.inner(area);

        frame.render_widget(Clear, area);
        frame.render_widget(block, area);

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(1), Constraint::Fill(1)])
            .split(inner_area);
        let colors = &self.app_data.borrow().theme.colors;
        frame.render_widget(
            Paragraph::new(self.header.clone()).fg(colors.side_select.normal.fg),
            layout[0],
        );
        self.select.draw(frame, layout[1]);
    }

    fn get_positioned_block(&mut self) -> Block<'_> {
        let colors = &self.app_data.borrow().theme.colors;
        let block = Block::new()
            .border_set(border::Set {
                vertical_left: "",
                vertical_right: "",
                ..border::EMPTY
            })
            .border_style(Style::default().fg(colors.side_select.normal.bg).bg(Color::Reset))
            .style(Style::default().bg(colors.side_select.normal.bg));

        if self.position == Position::Left {
            block.borders(Borders::LEFT)
        } else {
            block.borders(Borders::RIGHT)
        }
    }

    fn get_positioned_area(&self, area: Rect) -> Rect {
        let layout = Layout::default().direction(Direction::Horizontal);

        if self.position == Position::Left {
            layout
                .constraints([Constraint::Length(self.width), Constraint::Fill(1)])
                .split(area)[0]
        } else {
            layout
                .constraints([Constraint::Fill(1), Constraint::Length(self.width)])
                .split(area)[1]
        }
    }
}

impl<T: Table> Responsive for SideSelect<T> {
    fn process_event(&mut self, event: &TuiEvent) -> ResponseEvent {
        if !self.is_visible {
            return ResponseEvent::NotHandled;
        }

        if (self.app_data.has_binding(event, KeyCommand::SelectorLeft) && self.position == Position::Right)
            || (self.app_data.has_binding(event, KeyCommand::SelectorRight) && self.position == Position::Left)
            || self.app_data.has_binding(event, KeyCommand::NavigateBack)
        {
            self.is_visible = false;
            return ResponseEvent::Handled;
        }

        if self.app_data.has_binding(event, KeyCommand::SelectorLeft)
            || self.app_data.has_binding(event, KeyCommand::SelectorRight)
        {
            if self.is_key_pressed || self.showup_time.elapsed().as_millis() > 500 {
                self.is_visible = false;
            } else {
                self.select.items.highlight_first_item();
                self.is_key_pressed = true;
            }

            return ResponseEvent::Handled;
        }

        self.is_key_pressed = true;

        let mut navigate_into = false;
        if let Some(line_no) = event.get_clicked_line_no(self.select.area) {
            self.select.items.highlight_item_by_line(line_no);
            navigate_into = true;
        }

        if navigate_into || self.app_data.has_binding(event, KeyCommand::NavigateInto) {
            self.is_visible = false;
            if let Some(selected_name) = self.select.items.get_highlighted_item_name() {
                return (self.result)(selected_name.to_owned());
            }

            return ResponseEvent::Handled;
        }

        self.select.process_event(event);
        ResponseEvent::Handled
    }
}
