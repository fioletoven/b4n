use b4n_tui::{MouseEvent, MouseEventKind, TuiEvent};
use ratatui::buffer::Buffer;
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

    /// Process UI key/mouse event when there is a [`Screen`] available.
    pub fn process_screen_event(&mut self, event: &TuiEvent, screen: &Screen, area: Rect) {
        self.process_event_with_content(event, screen, area);
    }

    /// Process UI key/mouse event when there is a ratatui buffer available.
    pub fn process_buffer_event(&mut self, event: &TuiEvent, buffer: &Buffer, area: Rect) {
        let content = BufferContent::new(buffer, area);
        self.process_event_with_content(event, &content, area);
    }

    fn process_event_with_content(&mut self, event: &TuiEvent, content: &impl SelectableContent, area: Rect) {
        match event {
            TuiEvent::Key(_) => {
                self.start = None;
                self.end = None;
            },
            TuiEvent::Mouse(mouse) => self.process_mouse_event(*mouse, content, area),
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

    fn process_mouse_event(&mut self, mouse: MouseEvent, content: &impl SelectableContent, area: Rect) {
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
                let word_bounds = find_word_bounds(content, x, y);
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
    fn render(self, area: Rect, buf: &mut Buffer)
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

/// Content that can be used by [`ScreenSelection`].
trait SelectableContent {
    fn is_word_char_at(&self, x: u16, y: u16) -> bool;
    fn width(&self) -> u16;
}

impl SelectableContent for Screen {
    fn is_word_char_at(&self, x: u16, y: u16) -> bool {
        self.cell(y, x)
            .and_then(|cell| cell.contents().chars().next())
            .is_some_and(|ch| !ch.is_whitespace())
    }

    fn width(&self) -> u16 {
        self.size().1
    }
}

/// Represents buffer together with its area.
pub struct BufferContent<'a> {
    buffer: &'a Buffer,
    area: Rect,
}

impl<'a> BufferContent<'a> {
    /// Creates new [`BufferContent`] instance.
    pub fn new(buffer: &'a Buffer, area: Rect) -> Self {
        Self { buffer, area }
    }

    /// Extract text content from a buffer between two positions.
    pub fn contents_between(&self, start_row: u16, start_col: u16, end_row: u16, end_col: u16) -> String {
        buffer_contents_between(self.buffer, self.area, start_row, start_col, end_row, end_col)
    }

    /// Extract text content from a buffer.
    pub fn contents(&self) -> String {
        buffer_contents(self.buffer, self.area)
    }
}

impl SelectableContent for BufferContent<'_> {
    fn is_word_char_at(&self, x: u16, y: u16) -> bool {
        if x >= self.area.width || y >= self.area.height {
            return false;
        }

        let x = self.area.x.saturating_add(x);
        let y = self.area.y.saturating_add(y);

        self.buffer
            .cell((x, y))
            .map(|cell| cell.symbol())
            .and_then(|s| s.chars().next())
            .is_some_and(|ch| !ch.is_whitespace())
    }

    fn width(&self) -> u16 {
        self.area.width
    }
}

fn is_sorted(p1: Position, p2: Position) -> bool {
    p2.y > p1.y || (p2.y == p1.y && p2.x >= p1.x)
}

fn sort(p1: Position, p2: Position) -> (Position, Position) {
    if is_sorted(p1, p2) { (p1, p2) } else { (p2, p1) }
}

fn find_word_bounds(content: &impl SelectableContent, x: u16, y: u16) -> (u16, u16) {
    if !content.is_word_char_at(x, y) {
        return (x, x);
    }

    let screen_width = content.width();

    let mut start = x;
    while start > 0 && content.is_word_char_at(start - 1, y) {
        start -= 1;
    }

    let mut end = x;
    while end + 1 < screen_width && content.is_word_char_at(end + 1, y) {
        end += 1;
    }

    (start, end)
}

fn buffer_contents(buffer: &Buffer, area: Rect) -> String {
    let height = area.height.saturating_sub(1);
    let width = area.width.saturating_sub(1);
    buffer_contents_between(buffer, area, 0, 0, height, width)
}

fn buffer_contents_between(buffer: &Buffer, area: Rect, start_y: u16, start_x: u16, end_y: u16, end_x: u16) -> String {
    let mut result = String::new();

    let start_y = start_y.min(area.height.saturating_sub(1));
    let end_y = end_y.min(area.height.saturating_sub(1));

    for y in start_y..=end_y {
        let (line_start_x, line_end_x) = if start_y == end_y {
            (start_x, end_x)
        } else if y == start_y {
            (start_x, area.width)
        } else if y == end_y {
            (0, end_x)
        } else {
            (0, area.width)
        };

        let line_start_x = line_start_x.min(area.width.saturating_sub(1));
        let line_end_x = line_end_x.min(area.width);
        let abs_y = area.y.saturating_add(y);

        let mut whitespace_count = 0;
        for x in line_start_x..line_end_x {
            let abs_x = area.x.saturating_add(x);

            if let Some(cell) = buffer.cell((abs_x, abs_y)) {
                let symbol = cell.symbol();

                if symbol.chars().all(|c| c.is_whitespace()) {
                    whitespace_count += 1;
                } else {
                    result.extend(std::iter::repeat_n(' ', whitespace_count));
                    whitespace_count = 0;
                    result.push_str(symbol);
                }
            }
        }

        if y < end_y && !result.is_empty() {
            result.push('\n');
        }
    }

    while result.ends_with("\n\n") {
        result.pop();
    }

    result
}
