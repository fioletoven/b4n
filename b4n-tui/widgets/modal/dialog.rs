use b4n_config::themes::TextColors;
use crossterm::event::KeyCode;
use ratatui_core::layout::{Constraint, Direction, Layout, Position, Rect};
use ratatui_core::style::{Style, Stylize};
use ratatui_core::terminal::Frame;
use ratatui_core::text::Line;
use ratatui_widgets::block::Block;
use ratatui_widgets::clear::Clear;
use ratatui_widgets::paragraph::Paragraph;
use textwrap::Options;

use crate::widgets::{Button, CheckBox, ControlsGroup, Selector, TextBox};
use crate::{MouseEventKind, ResponseEvent, Responsive, TuiEvent, utils::center};

type OnChangeFn = Box<dyn FnMut(&mut String, &mut ControlsGroup)>;

/// UI modal dialog.
pub struct Dialog {
    pub is_visible: bool,
    width: u16,
    colors: TextColors,
    message: String,
    controls: ControlsGroup,
    default_button: usize,
    on_change: Option<OnChangeFn>,
    area: Rect,
}

impl Default for Dialog {
    fn default() -> Self {
        Self::new(String::new(), Vec::new())
    }
}

impl Dialog {
    /// Creates new [`Dialog`] instance.
    pub fn new(message: String, buttons: Vec<Button>) -> Self {
        let default_button = if buttons.is_empty() { 0 } else { buttons.len() - 1 };
        let mut buttons = ControlsGroup::new(buttons);
        buttons.focus(default_button);

        Self {
            is_visible: false,
            width: 60,
            colors: TextColors::default(),
            message,
            controls: buttons,
            default_button,
            on_change: None,
            area: Rect::default(),
        }
    }

    /// Sets dialog width.
    pub fn with_width(mut self, width: u16) -> Self {
        self.width = width;
        self
    }

    /// Sets dialog colors.
    pub fn with_colors(mut self, colors: TextColors) -> Self {
        self.colors = colors;
        self
    }

    /// Highlights item under the specified mouse position on the first dialog draw.
    pub fn with_highlighted_position(mut self, position: Option<Position>) -> Self {
        self.controls.highlighted_position(position);
        self
    }

    /// Sets provided textboxes for the dialog.
    pub fn with_textboxes(mut self, textboxes: Vec<TextBox>) -> Self {
        for textbox in textboxes {
            self.controls.add_textbox(textbox);
        }

        self
    }

    /// Sets provided checkboxes for the dialog.
    pub fn with_checkboxes(mut self, checkboxes: Vec<CheckBox>) -> Self {
        for checkbox in checkboxes {
            self.controls.add_checkbox(checkbox);
        }

        self
    }

    /// Sets provided selectors for the dialog.
    pub fn with_selectors(mut self, selectors: Vec<Selector>) -> Self {
        for selector in selectors {
            self.controls.add_selector(selector);
        }

        self
    }

    /// Sets `on_change` action for the dialog.
    pub fn with_on_change<F: FnMut(&mut String, &mut ControlsGroup) + 'static>(mut self, action: F) -> Self {
        self.on_change = Some(Box::new(action));
        self
    }

    /// Returns textbox under specified `id`.
    pub fn textbox(&self, id: usize) -> Option<&TextBox> {
        self.controls.textbox(id)
    }

    /// Returns textbox under specified `id`.
    pub fn textbox_mut(&mut self, id: usize) -> Option<&mut TextBox> {
        self.controls.textbox_mut(id)
    }

    /// Returns checkbox under specified `id`.
    pub fn checkbox(&self, id: usize) -> Option<&CheckBox> {
        self.controls.checkbox(id)
    }

    /// Returns selector under specified `id`.
    pub fn selector(&self, id: usize) -> Option<&Selector> {
        self.controls.selector(id)
    }

    /// Marks [`Dialog`] as a visible.
    pub fn show(&mut self) {
        self.is_visible = true;
    }

    /// Draws [`Dialog`] on the provided frame area.
    pub fn draw(&mut self, frame: &mut Frame<'_>, area: Rect) {
        if !self.is_visible {
            return;
        }

        let width = std::cmp::min(area.width, self.width).max(2) - 2;
        let text = textwrap::wrap(
            &self.message,
            Options::new(width.into())
                .break_words(false)
                .initial_indent("  ")
                .subsequent_indent("  "),
        );
        let lines = u16::try_from(self.controls.controls().len()).unwrap_or_default();
        let lines = if lines == 0 { 3 } else { lines + 4 };
        let height = u16::try_from(text.len()).unwrap_or_default() + lines + 1;

        self.area = center(area, Constraint::Length(self.width), Constraint::Length(height));
        let block = Block::new().style(Style::default().bg(self.colors.bg));

        frame.render_widget(Clear, self.area);
        frame.render_widget(block, self.area);

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(1), Constraint::Fill(1), Constraint::Length(lines)])
            .split(self.area);

        let lines: Vec<Line> = text.iter().map(|i| Line::from(i.as_ref())).collect();
        frame.render_widget(Paragraph::new(lines).fg(self.colors.fg), layout[1]);

        self.controls.draw(frame, layout[2]);
    }
}

impl Responsive for Dialog {
    fn process_event(&mut self, event: &TuiEvent) -> ResponseEvent {
        if !self.is_visible {
            return ResponseEvent::NotHandled;
        }

        if !self.controls.has_opened_selector()
            && (matches!(event, TuiEvent::Key(key) if key.code == KeyCode::Esc)
                || event.is_out(MouseEventKind::LeftClick, self.area))
        {
            self.is_visible = false;
            return self.controls.result(self.default_button);
        }

        let (result, is_button) = self.controls.process_event(event);
        if result == ResponseEvent::Changed {
            if let Some(ref mut callback) = self.on_change {
                callback(&mut self.message, &mut self.controls);
            }

            return ResponseEvent::Handled;
        }

        // we must close dialog if:
        //  - button returned anything else than Handled (user can configure any response)
        //  - control returned anything else than Handled or Action (user can configure actions for TextBox buttons)
        if (is_button && result != ResponseEvent::Handled) || !matches!(result, ResponseEvent::Handled | ResponseEvent::Action(_))
        {
            self.is_visible = false;
        }

        result
    }
}
