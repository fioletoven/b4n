use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    symbols::border,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};
use std::{rc::Rc, time::Instant};

use crate::{
    app::SharedAppData,
    ui::{ResponseEvent, Responsive, Table},
};

use super::Input;

/// Possible positions for the [`Selector`] widget
#[derive(PartialEq)]
pub enum SelectorPosition {
    Left,
    Right,
}

/// Selector widget for TUI
pub struct Selector<T: Table> {
    pub is_visible: bool,
    pub items: T,
    app_data: SharedAppData,
    header: String,
    position: SelectorPosition,
    result: fn(String) -> ResponseEvent,
    width: u16,
    filter: Input,
    is_key_pressed: bool,
    showup_time: Instant,
}

impl<T: Table> Selector<T> {
    /// Creates new [`Selector`] instance
    pub fn new(
        name: &str,
        app_data: SharedAppData,
        list: T,
        position: SelectorPosition,
        result: fn(String) -> ResponseEvent,
        width: u16,
    ) -> Self {
        let header = format!(" SELECT {}: ", name);
        let colors = app_data.borrow().config.theme.colors;

        Selector {
            is_visible: false,
            items: list,
            app_data,
            header,
            position,
            result,
            width: std::cmp::max(width, 5),
            filter: Input::new(
                Style::default().fg(colors.selector.input.fg).bg(colors.selector.input.bg),
                false,
            ),
            is_key_pressed: false,
            showup_time: Instant::now(),
        }
    }

    /// Marks selector as visible, after that it can be drawn on terminal frame
    pub fn show(&mut self) {
        self.is_key_pressed = false;
        self.is_visible = true;
        self.filter.reset();
        self.items.filter(None);
        self.showup_time = Instant::now();
    }

    /// Marks selector as visible and selects item by name
    pub fn show_selected(&mut self, selected_name: &str) {
        self.items.filter(None);
        self.items.highlight_item_by_name(selected_name);
        self.show();
    }

    /// Draws [`Selector`] on the provided frame area
    pub fn draw(&mut self, frame: &mut ratatui::Frame<'_>, area: Rect) {
        if !self.is_visible {
            return;
        }

        let area = self.get_positioned_area(area);
        let block = self.get_positioned_block();
        let inner_area = block.inner(area);

        frame.render_widget(Clear, area);
        frame.render_widget(block, area);

        let layout = get_layout(inner_area, self.items.get_filter().is_some());
        let colors = &self.app_data.borrow().config.theme.colors;
        frame.render_widget(Paragraph::new(self.header.clone()).fg(colors.selector.normal.fg), layout[0]);

        if self.items.get_filter().is_some() {
            self.filter.draw(frame, layout[1]);
        }

        let list_area = if self.items.get_filter().is_some() {
            layout[2]
        } else {
            layout[1]
        };
        self.items.update_page(list_area.height);
        if let Some(list) = self.items.get_paged_names(usize::from(self.width - 3)) {
            frame.render_widget(Paragraph::new(self.get_resource_names(list)), list_area);
        }
    }

    /// Returns resource names as formatted rows
    fn get_resource_names<'a>(&self, resources: Vec<(String, bool)>) -> Vec<Line<'a>> {
        let mut result = Vec::with_capacity(resources.len());
        let colors = &self.app_data.borrow().config.theme.colors;

        for (name, is_active) in resources {
            let colors = if is_active {
                colors.selector.normal_hl
            } else {
                colors.selector.normal
            };
            let row = Span::styled(name, Style::new().fg(colors.fg).bg(colors.bg));
            result.push(Line::from(vec![Span::raw(" "), row, Span::raw("\n")]));
        }

        result
    }

    fn get_positioned_block(&mut self) -> Block<'_> {
        let colors = &self.app_data.borrow().config.theme.colors;
        let block = Block::new()
            .border_set(border::Set {
                vertical_left: "",
                vertical_right: "",
                ..border::EMPTY
            })
            .border_style(Style::default().fg(colors.selector.normal.bg).bg(Color::Reset))
            .style(Style::default().bg(colors.selector.normal.bg));

        if self.position == SelectorPosition::Left {
            block.borders(Borders::LEFT)
        } else {
            block.borders(Borders::RIGHT)
        }
    }

    fn get_positioned_area(&self, area: Rect) -> Rect {
        let layout = Layout::default().direction(Direction::Horizontal);

        if self.position == SelectorPosition::Left {
            layout
                .constraints([Constraint::Length(self.width), Constraint::Fill(1)])
                .split(area)[0]
        } else {
            layout
                .constraints([Constraint::Fill(1), Constraint::Length(self.width)])
                .split(area)[1]
        }
    }

    fn filter_and_highlight(&mut self) {
        self.items.filter(Some(self.filter.value().to_owned()));
        self.items.highlight_item_by_name_start(self.filter.value());
        if self.items.get_highlighted_item_index().is_none() {
            self.items.highlight_first_item();
        }
    }
}

impl<T: Table> Responsive for Selector<T> {
    fn process_key(&mut self, key: KeyEvent) -> ResponseEvent {
        if !self.is_visible {
            return ResponseEvent::NotHandled;
        }

        if (key.code == KeyCode::Left && self.position == SelectorPosition::Right)
            || (key.code == KeyCode::Right && self.position == SelectorPosition::Left || key.code == KeyCode::Esc)
        {
            self.is_visible = false;
            return ResponseEvent::Handled;
        }

        if key.code == KeyCode::Left || key.code == KeyCode::Right {
            if self.is_key_pressed || self.showup_time.elapsed().as_millis() > 500 {
                self.is_visible = false;
            } else {
                self.items.highlight_first_item();
                self.is_key_pressed = true;
            }

            return ResponseEvent::Handled;
        }

        self.is_key_pressed = true;

        if key.code == KeyCode::Enter {
            self.is_visible = false;
            if let Some(selected_name) = self.items.get_highlighted_item_name() {
                return (self.result)(selected_name.to_owned());
            }

            return ResponseEvent::Handled;
        }

        if self.items.process_key(key) == ResponseEvent::NotHandled {
            self.filter.process_key(key);
            if self.filter.value().is_empty() {
                self.items.filter(None);
            } else {
                if let Some(filter) = self.items.get_filter() {
                    if self.filter.value() != filter {
                        self.filter_and_highlight();
                    }
                } else {
                    self.filter_and_highlight();
                }
            }
        }

        ResponseEvent::Handled
    }
}

fn get_layout(area: Rect, is_filter_shown: bool) -> Rc<[Rect]> {
    let constraints = if is_filter_shown {
        vec![Constraint::Length(1), Constraint::Length(1), Constraint::Fill(1)]
    } else {
        vec![Constraint::Length(1), Constraint::Fill(1)]
    };

    Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area)
}
