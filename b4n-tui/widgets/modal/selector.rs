use b4n_config::keys::KeyCombination;
use b4n_config::themes::{ControlColors, SelectColors, TextColors};
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui_core::layout::{Margin, Position, Rect};
use ratatui_core::terminal::Frame;
use ratatui_core::text::Line;
use ratatui_widgets::block::Block;
use ratatui_widgets::clear::Clear;
use ratatui_widgets::paragraph::Paragraph;

use crate::table::Table;
use crate::widgets::{ActionsList, ActionsListBuilder, Select};
use crate::{MouseEventKind, ResponseEvent, Responsive, TuiEvent};

/// UI `Selector`.
pub struct Selector {
    pub id: usize,
    is_focused: bool,
    is_selecting: bool,
    caption: &'static str,
    caption_width: usize,
    options: Select<ActionsList>,
    options_width: usize,
    normal: TextColors,
    focused: TextColors,
    selected: String,
    area: Rect,
    width: u16,
}

impl Selector {
    /// Creates new [`Selector`] instance.
    pub fn new(id: usize, caption: &'static str, options: &[&str], select: SelectColors, control: &ControlColors) -> Self {
        let mut options = ActionsListBuilder::from_strings(options).build(None);
        options.highlight_first_item();
        let selected = options.get_highlighted_item_name().unwrap_or_default().to_owned();

        let mut options = Select::new(options, select, false, false);
        options.disable_filter(true);

        let caption_width = caption.chars().count();
        let options_width = options
            .items
            .list
            .items
            .as_ref()
            .and_then(|l| l.full_iter().map(|i| i.data.name.chars().count()).max())
            .unwrap_or_default();

        Self {
            id,
            is_focused: false,
            is_selecting: false,
            caption,
            caption_width,
            options,
            options_width,
            normal: control.normal,
            focused: control.focused,
            selected,
            area: Rect::default(),
            width: u16::try_from(caption_width + options_width).unwrap_or_default() + 6,
        }
    }

    /// Returns selected option.
    pub fn selected(&self) -> &str {
        &self.selected
    }

    /// Returns `true` if provided `x` and `y` are inside the selector.
    pub fn contains(&self, x: u16, y: u16) -> bool {
        self.area.contains(Position::new(x, y))
    }

    /// Returns `true` if selector is focused and in selecting state.
    pub fn is_opened(&self) -> bool {
        self.is_focused && self.is_selecting
    }

    /// Returns `true` if selector is focused.
    pub fn is_focused(&self) -> bool {
        self.is_focused
    }

    /// Activates or deactivates selector.
    pub fn set_focus(&mut self, is_active: bool) {
        self.is_focused = is_active;
    }

    /// Process selector click.
    pub fn click(&mut self, position: Option<Position>) -> ResponseEvent {
        self.is_selecting = true;

        let area = self.get_options_area();
        if let Some(position) = position
            && area.contains(position)
        {
            self.options.items.highlight_item_by_line(position.y.saturating_sub(area.y));
        } else {
            self.options.items.highlight_item_by_name(&self.selected);
        }

        ResponseEvent::Handled
    }

    /// Draws [`Selector`] on the provided frame area.
    pub fn draw(&mut self, frame: &mut Frame<'_>, area: Rect) {
        let area = area.inner(Margin::new(5, 0));
        let colors = if self.is_focused { self.focused } else { self.normal };
        let icon = if self.is_opened() { '' } else { '' };
        let text = format!(" {} {}: {} ", icon, self.caption, self.selected);
        let line = Line::styled(text, &colors);

        frame.render_widget(Paragraph::new(line), area);

        self.area = area;
        self.area.width = self.width;
    }

    /// Draws selector options on the provided frame.
    pub fn draw_options(&mut self, frame: &mut Frame<'_>) {
        if self.is_opened() {
            let area = self.get_options_area();
            frame.render_widget(Clear, area);
            frame.render_widget(Block::new().style(&self.options.colors().normal), area);
            self.options.draw(frame, area.inner(Margin::new(1, 0)));
        }
    }

    fn get_options_area(&self) -> Rect {
        Rect::new(
            self.area.x + u16::try_from(self.caption_width).unwrap_or_default() + 4,
            self.area.y,
            u16::try_from(self.options_width).unwrap_or_default() + 2,
            u16::try_from(self.options.items.len()).unwrap_or_default(),
        )
    }
}

impl Responsive for Selector {
    fn process_event(&mut self, event: &TuiEvent) -> ResponseEvent {
        if !self.is_opened() {
            return ResponseEvent::NotHandled;
        }

        if event.is_key(&KeyCombination::new(KeyCode::Tab, KeyModifiers::empty())) {
            self.is_selecting = false;
            return ResponseEvent::NotHandled;
        }

        let area = self.get_options_area();
        if event.is_key(&KeyCombination::new(KeyCode::Esc, KeyModifiers::empty()))
            || event.is_out(MouseEventKind::LeftClick, area)
        {
            self.is_selecting = false;
            return ResponseEvent::Handled;
        }

        if event.is_key(&KeyCombination::new(KeyCode::Enter, KeyModifiers::empty()))
            || event.is_key(&KeyCombination::new(KeyCode::Char(' '), KeyModifiers::empty()))
            || event.is_in(MouseEventKind::LeftClick, area)
        {
            self.selected = self.options.items.get_highlighted_item_name().unwrap_or_default().to_owned();
            self.is_selecting = false;
            return ResponseEvent::Handled;
        }

        self.options.process_event(event)
    }
}
