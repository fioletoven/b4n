use b4n_config::keys::KeyCommand;
use b4n_tui::widgets::Select;
use b4n_tui::{MouseEventKind, ResponseEvent, Responsive, TuiEvent, table::Table};
use crossterm::event::KeyModifiers;
use ratatui::layout::{Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::symbols::border;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use std::time::Instant;

use crate::core::{SharedAppData, SharedAppDataExt};

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
    result: Option<fn(String) -> ResponseEvent>,
    width: u16,
    item_to_highlight: &'static str,
    is_key_pressed: bool,
    showup_time: Instant,
}

impl<T: Table> SideSelect<T> {
    /// Creates new [`SideSelect`] instance.
    pub fn new(app_data: SharedAppData, list: T, position: Position, width: u16) -> Self {
        let select = Select::new(list, app_data.borrow().theme.colors.side_select.clone(), true, false);

        SideSelect {
            is_visible: false,
            select,
            app_data,
            header: " SELECT ".to_owned(),
            position,
            result: None,
            width: std::cmp::max(width, 5),
            item_to_highlight: "",
            is_key_pressed: false,
            showup_time: Instant::now(),
        }
    }

    /// Sets new name for the side select.
    pub fn with_name(mut self, name: &str) -> Self {
        self.header = format!(" SELECT {name}: ");
        self
    }

    /// Sets function that is called to obtain [`ResponseEvent`].
    pub fn with_result(mut self, result: fn(String) -> ResponseEvent) -> Self {
        self.result = Some(result);
        self
    }

    /// Sets name of the item to highlight on double key press.
    pub fn with_quick_highlight(mut self, name: &'static str) -> Self {
        self.item_to_highlight = name;
        self
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

    /// Marks [`SideSelect`] as visible and highlights an item by name.
    pub fn show_selected(&mut self, selected_name: &str) {
        self.select.highlight(selected_name);
        self.show();
    }

    /// Marks [`SideSelect`] as visible and highlights an item by uid.
    pub fn show_selected_uid(&mut self, selected_uid: &str) {
        self.select.highlight_by_uid(selected_uid);
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

        self.select.draw(frame, layout[1].inner(Margin::new(1, 0)));
    }

    /// Returns width of the [`SideSelect`].
    pub fn width(&self) -> u16 {
        self.width
    }

    fn get_positioned_block(&mut self) -> Block<'_> {
        let colors = &self.app_data.borrow().theme.colors;
        let block = Block::new()
            .border_set(border::Set {
                vertical_left: "",
                vertical_right: "",
                ..border::EMPTY
            })
            .border_style(Style::default().fg(colors.side_select.normal.bg).bg(colors.text.bg))
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
            || event.is_out(MouseEventKind::LeftClick, self.select.area)
            || event.is_out(MouseEventKind::RightClick, self.select.area)
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
                if self.item_to_highlight.is_empty() {
                    self.select.items.highlight_first_item();
                } else {
                    self.select.items.highlight_item_by_name(self.item_to_highlight);
                }

                self.is_key_pressed = true;
            }

            return ResponseEvent::Handled;
        }

        self.is_key_pressed = true;

        let mut navigate_into = false;
        if let Some(line_no) = event.get_line_no(MouseEventKind::LeftClick, KeyModifiers::NONE, self.select.area) {
            self.select.items.highlight_item_by_line(line_no);
            navigate_into = true;
        }

        if navigate_into || self.app_data.has_binding(event, KeyCommand::NavigateInto) {
            self.is_visible = false;
            if let Some(result_fn) = self.result
                && let Some(selected_name) = self.select.items.get_highlighted_item_name()
            {
                return result_fn(selected_name.to_owned());
            }

            return ResponseEvent::Handled;
        }

        self.select.process_event(event);
        ResponseEvent::Handled
    }
}
