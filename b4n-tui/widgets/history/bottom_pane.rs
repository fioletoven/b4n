use b4n_config::keys::KeyCombination;
use b4n_config::themes::Theme;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::{Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::{Color, Style, Stylize};
use ratatui::symbols::border;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use textwrap::Options;

use crate::table::Table;
use crate::widgets::history::MessageItem;
use crate::widgets::{List, history::MessagesList};
use crate::{MouseEventKind, ResponseEvent, Responsive, TuiEvent};

/// Footer bottom history pane.
pub struct BottomPane {
    history: List<MessagesList>,
    area: Rect,
}

impl BottomPane {
    /// Creates new [`BottomPane`] instance.
    pub fn new(messages: MessagesList) -> Self {
        Self {
            history: List::new(messages),
            area: Rect::default(),
        }
    }

    /// Updates [`BottomPane`] with new messages list.
    pub fn update(&mut self, messages: Vec<MessageItem>) {
        self.history.items.update(messages);
    }

    /// Draws [`BottomPane`] on the provided frame area.
    pub fn draw(&mut self, frame: &mut ratatui::Frame<'_>, area: Rect, theme: &Theme) {
        let hint_lines = if let Some(text) = self.history.items.get_highlighted_item_name() {
            let width = area.width.saturating_sub(4);
            let text = textwrap::wrap(text, Options::new(width.into()).initial_indent(" "));
            text.into_iter().map(|i| Line::from(i.into_owned())).collect::<Vec<Line>>()
        } else {
            Vec::default()
        };
        let show_hint =
            !hint_lines.is_empty() && (hint_lines.len() > 1 || hint_lines[0].width() >= area.width.saturating_sub(9).into());
        let hint_height = if show_hint {
            u16::try_from(hint_lines.len()).unwrap_or_default()
        } else {
            0
        };

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Fill(1),
                Constraint::Length(hint_height),
                Constraint::Length(10),
            ])
            .split(area);
        self.area = layout[1].union(layout[2]);

        if show_hint {
            let block = get_block(theme.colors.footer.hint.bg, theme.colors.text.bg);
            let inner_area = block.inner(layout[1]).inner(Margin::new(1, 0));
            frame.render_widget(Clear, layout[1]);
            frame.render_widget(block, layout[1]);
            frame.render_widget(Paragraph::new(hint_lines).fg(theme.colors.footer.hint.fg), inner_area);
        }

        let block = get_block(theme.colors.footer.text.bg, theme.colors.text.bg);
        let inner_area = block.inner(layout[2]).inner(Margin::new(1, 0));

        frame.render_widget(Clear, layout[2]);
        frame.render_widget(block, layout[2]);

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

fn get_block(bg: Color, app_bg: Color) -> Block<'static> {
    Block::new()
        .border_set(border::Set {
            vertical_left: "",
            vertical_right: "",
            ..border::EMPTY
        })
        .borders(Borders::LEFT | Borders::RIGHT)
        .border_style(Style::default().fg(bg).bg(app_bg))
        .style(Style::default().bg(bg))
}
