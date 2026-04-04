use b4n_tui::{MouseEvent, MouseEventKind, TuiEvent};
use ratatui::layout::{Position, Rect};
use ratatui::style::Color;
use ratatui::widgets::Widget;
use tui_term::vt100::Screen;

/// Holds simple selection data for the TUI screen.
#[derive(Default)]
pub struct ScreenSelection {
    start: Option<Position>,
    end: Option<Position>,
    sorted: Option<(Position, Position)>,
    color: Color,
}

impl ScreenSelection {
    /// Sets selection color.
    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Resets selection.
    pub fn reset(&mut self) {
        self.start = None;
        self.end = None;
        self.sorted = None;
    }

    /// Returns sorted start and end for the current selection.
    pub fn sorted(&self) -> Option<(Position, Position)> {
        self.sorted
    }

    /// Process UI key/mouse event.
    pub fn process_event(&mut self, event: &TuiEvent, screen: &Screen, area: Rect) {
        match event {
            TuiEvent::Key(_) => {
                self.start = None;
                self.end = None;
            },
            TuiEvent::Mouse(mouse) => self.process_mouse_event(*mouse, screen, area),
            TuiEvent::Command(_) => (),
        }

        if let Some(start) = self.start
            && let Some(end) = self.end
        {
            self.sorted = Some(sort(start, end));
        } else {
            self.sorted = None;
        }
    }

    fn process_mouse_event(&mut self, mouse: MouseEvent, screen: &Screen, area: Rect) {
        if !area.contains((mouse.column, mouse.row).into()) {
            return;
        }

        let x = mouse.column.saturating_sub(area.x);
        let y = mouse.row.saturating_sub(area.y);

        match mouse.kind {
            MouseEventKind::LeftClick => {
                self.start = Some(Position::new(x, y));
                self.end = None;
            },
            MouseEventKind::LeftDrag => {
                self.end = Some(Position::new(x, y));
            },
            MouseEventKind::LeftDoubleClick => {
                let word_bounds = find_word_bounds(screen, x, y);
                self.start = Some(Position::new(word_bounds.0, y));
                self.end = Some(Position::new(word_bounds.1, y));
            },
            MouseEventKind::LeftTripleClick => {
                self.start = Some(Position::new(0, y));
                self.end = Some(Position::new(area.width.saturating_sub(1), y));
            },
            _ => (),
        }
    }
}

impl Widget for &ScreenSelection {
    fn render(self, area: Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let Some((start, end)) = self.sorted else {
            return;
        };

        for current_line in start.y..=end.y {
            let (draw_from, draw_to) = if start.y == end.y {
                (start.x, end.x)
            } else if current_line == start.y {
                (start.x, area.width.saturating_sub(1))
            } else if current_line == end.y {
                (0, end.x)
            } else {
                (0, area.width.saturating_sub(1))
            };

            for x in draw_from..=draw_to {
                buf[(area.x + x, area.y + current_line)].bg = self.color;
            }
        }
    }
}

fn is_sorted(p1: Position, p2: Position) -> bool {
    p2.y > p1.y || (p2.y == p1.y && p2.x >= p1.x)
}

fn sort(p1: Position, p2: Position) -> (Position, Position) {
    if is_sorted(p1, p2) { (p1, p2) } else { (p2, p1) }
}

fn find_word_bounds(screen: &Screen, x: u16, y: u16) -> (u16, u16) {
    if !is_word_char(screen, x, y) {
        return (x, x);
    }

    let screen_width = screen.size().1;

    let mut start = x;
    while start > 0 && is_word_char(screen, start - 1, y) {
        start -= 1;
    }

    let mut end = x;
    while end + 1 < screen_width && is_word_char(screen, end + 1, y) {
        end += 1;
    }

    (start, end)
}

fn is_word_char(screen: &Screen, x: u16, y: u16) -> bool {
    screen
        .cell(y, x)
        .and_then(|cell| cell.contents().chars().next())
        .map(|ch| !ch.is_whitespace())
        .unwrap_or(false)
}
