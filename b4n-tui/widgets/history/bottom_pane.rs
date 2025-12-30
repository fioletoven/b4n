use b4n_config::keys::KeyCombination;
use b4n_config::themes::Theme;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::{Margin, Rect};
use ratatui::style::Style;
use ratatui::symbols::border;
use ratatui::widgets::{Block, Borders, Clear};

use crate::widgets::{List, history::MessagesList};
use crate::{MouseEventKind, ResponseEvent, Responsive, TuiEvent};

pub struct BottomPane {
    history: List<MessagesList>,
    area: Rect,
}

impl BottomPane {
    pub fn new(messages: MessagesList) -> Self {
        Self {
            history: List::new(messages),
            area: Rect::default(),
        }
    }

    /// Draws [`BottomPane`] on the provided frame area.
    pub fn draw(&mut self, frame: &mut ratatui::Frame<'_>, area: Rect, theme: &Theme) {
        self.area = area;
        let block = Block::new()
            .border_set(border::Set {
                vertical_left: "",
                vertical_right: "",
                ..border::EMPTY
            })
            .borders(Borders::LEFT | Borders::RIGHT)
            .border_style(Style::default().fg(theme.colors.footer.text.bg).bg(theme.colors.text.bg))
            .style(Style::default().bg(theme.colors.footer.text.bg));
        let inner_area = block.inner(area).inner(Margin::new(1, 0));

        frame.render_widget(Clear, area);
        frame.render_widget(block, area);

        self.history.draw(frame, inner_area, theme);
    }
}

impl Responsive for BottomPane {
    fn process_event(&mut self, event: &TuiEvent) -> ResponseEvent {
        if event.is_out(MouseEventKind::LeftClick, self.area)
            || event.is_key(&KeyCombination::new(KeyCode::Esc, KeyModifiers::empty()))
        {
            return ResponseEvent::Cancelled;
        }

        self.history.process_event(event)
    }
}
