use b4n_common::DelayedTrueTracker;
use b4n_config::{keys::KeyCommand, themes::TextColors};
use b4n_tui::widgets::Spinner;
use b4n_tui::{MouseEventKind, ResponseEvent, Responsive, TuiEvent, table::Table, table::ViewType, utils::center};
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::{Constraint, Direction, Layout, Margin, Position, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph, Widget};

use crate::core::{SharedAppData, SharedAppDataExt};

/// List viewer.
pub struct ListViewer<T: Table> {
    pub table: T,
    pub view: ViewType,
    pub area: Rect,
    app_data: SharedAppData,
    has_error: DelayedTrueTracker,
    is_disconnected: DelayedTrueTracker,
    spinner: Spinner,
}

impl<T: Table> ListViewer<T> {
    /// Creates new [`ListViewer`] instance.
    pub fn new(app_data: SharedAppData, list: T, view: ViewType) -> Self {
        ListViewer {
            table: list,
            view,
            area: Rect::default(),
            app_data,
            has_error: DelayedTrueTracker::default(),
            is_disconnected: DelayedTrueTracker::default(),
            spinner: Spinner::default(),
        }
    }

    /// Draws [`ListViewer`] on the provided frame area.\
    /// It draws only the visible elements respecting the height of the `area`.
    pub fn draw(&mut self, frame: &mut ratatui::Frame<'_>, area: Rect) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(1), Constraint::Fill(1)])
            .split(area);
        self.area = layout[1].inner(Margin::new(1, 0));

        {
            let theme = &self.app_data.borrow().theme;
            frame.render_widget(Block::new().style(&theme.colors.text), area);

            self.table.refresh_header(self.view, usize::from(self.area.width));
            let sort_symbols = self.table.get_sort_symbols();
            let offset = self.table.refresh_offset();
            let mut header = HeaderWidget {
                header: self.table.get_header(self.view, usize::from(self.area.width)),
                offset,
                colors: &theme.colors.header.text,
                background: theme.colors.text.bg,
                view: self.view,
                sort_symbols: &sort_symbols,
            };
            frame.render_widget(&mut header, layout[0]);
        }

        self.table.update_page(self.area.height);

        self.is_disconnected.update(!self.app_data.borrow().is_connected);
        if !self.app_data.borrow().is_connected {
            if self.is_disconnected.value() {
                self.render_error(frame, " connecting to the Kubernetes cluster…", true);
            }
        } else if self.has_error.value() {
            self.render_error(frame, " cannot fetch or update requested resources…", false);
        } else {
            let theme = &self.app_data.borrow().theme;
            if let Some(list) = self.table.get_paged_items(theme, self.view, usize::from(self.area.width)) {
                frame.render_widget(Paragraph::new(get_items(&list)).style(&theme.colors.text), self.area);
            }
        }
    }

    /// Updates error state for the resources list.
    pub fn update_error_state(&mut self, has_error: bool) {
        self.has_error.update(has_error);
    }

    fn render_error(&mut self, frame: &mut ratatui::Frame<'_>, error: &str, has_spinner: bool) {
        let colors = &self.app_data.borrow().theme.colors;
        let spans = if has_spinner {
            vec![Span::raw(self.spinner.tick().to_string()), error.into()]
        } else {
            vec![error.into()]
        };
        let line = Line::default().spans(spans).style(&colors.text);
        let area = center(self.area, Constraint::Length(line.width() as u16), Constraint::Length(4));
        frame.render_widget(line, area);
    }
}

/// Returns formatted items rows.
fn get_items(items: &Vec<(String, TextColors)>) -> Vec<Line<'_>> {
    let mut result = Vec::with_capacity(items.len());

    for (text, colors) in items {
        result.push(Line::styled(text, colors));
    }

    result
}

impl<T: Table> Responsive for ListViewer<T> {
    fn process_event(&mut self, event: &TuiEvent) -> ResponseEvent {
        if self.has_error.value() {
            return ResponseEvent::NotHandled;
        }

        if let TuiEvent::Key(key) = event
            && key.code == KeyCode::Char('0')
            && key.modifiers == KeyModifiers::ALT
            && self.view != ViewType::Full
        {
            return ResponseEvent::Handled;
        }

        if self.table.process_event(event) == ResponseEvent::Handled {
            return ResponseEvent::Handled;
        }

        if self.app_data.has_binding(event, KeyCommand::NavigateSelect) {
            self.table.select_highlighted_item();
            return ResponseEvent::Handled;
        }

        if self.app_data.has_binding(event, KeyCommand::NavigateSelectAll) {
            self.table.select_all();
            return ResponseEvent::Handled;
        }

        if self.app_data.has_binding(event, KeyCommand::NavigateInvertSelection) {
            self.table.invert_selection();
            return ResponseEvent::Handled;
        }

        if let TuiEvent::Mouse(mouse) = event
            && mouse.kind == MouseEventKind::LeftClick
        {
            if self.area.contains(Position::new(mouse.column, mouse.row)) {
                // mouse click is inside list area
                let line_no = mouse.row.saturating_sub(self.area.y);
                if self.table.highlight_item_by_line(line_no) {
                    if mouse.modifiers == KeyModifiers::CONTROL {
                        self.table.select_highlighted_item();
                    }
                } else {
                    self.table.unhighlight_item();
                }

                return ResponseEvent::Handled;
            } else if Rect::new(self.area.x, self.area.y.saturating_sub(1), self.area.width, 1)
                .contains(Position::new(mouse.column, mouse.row))
            {
                // mouse click is inside header area
                let position = usize::from(mouse.column.saturating_sub(self.area.x)) + self.table.offset();
                if let Some(column_no) = self.table.get_column_at_position(position) {
                    let column_no = column_no
                        .saturating_add(if self.view == ViewType::Full { 0 } else { 1 })
                        .saturating_sub(1);

                    self.table.toggle_sort(column_no);
                }

                return ResponseEvent::Handled;
            }
        }

        ResponseEvent::NotHandled
    }
}

/// Widget that renders header for the items list pane.\
/// It underlines sort symbol inside each column name.
struct HeaderWidget<'a> {
    pub header: &'a str,
    pub offset: usize,
    pub colors: &'a TextColors,
    pub background: Color,
    pub view: ViewType,
    pub sort_symbols: &'a [char],
}

impl Widget for &mut HeaderWidget<'_> {
    fn render(self, area: Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let x = area.left() + 1;
        let y = area.top();
        let max_x = area.left() + buf.area.width - 1;

        buf[(x - 1, y)].set_char('').set_fg(self.colors.bg).set_bg(self.background);
        buf[(max_x, y)].set_char('').set_fg(self.colors.bg).set_bg(self.background);

        let mut column_no = if self.view == ViewType::Full { 0 } else { 1 };
        let mut in_column = false;
        let mut highlighted = false;

        for (i, char) in self.header.chars().enumerate() {
            let visible = i >= self.offset;
            let x = x + i.saturating_sub(self.offset) as u16;
            if x >= max_x {
                break;
            }

            if char != ' ' && !in_column {
                in_column = true;
                highlighted = false;
            } else if char == ' ' && in_column {
                in_column = false;
                column_no += 1;
            }

            let can_be_highlighted = column_no < self.sort_symbols.len()
                && self.sort_symbols[column_no] != ' '
                && char == self.sort_symbols[column_no];

            if in_column && can_be_highlighted && !highlighted {
                highlighted = true;
                if visible {
                    buf[(x, y)].set_style(Style::default().underlined());
                }
            }

            if !visible {
                continue;
            }

            if char == '↑' || char == '↓' {
                buf[(x, y)].set_char(char).set_fg(self.colors.dim).set_bg(self.colors.bg);
            } else {
                buf[(x, y)].set_char(char).set_fg(self.colors.fg).set_bg(self.colors.bg);
            }
        }
    }
}
