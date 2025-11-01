use b4n_common::{Icon, IconAction, IconKind, Notification, NotificationSink};
use b4n_config::themes::{Theme, ThemeColors};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Flex, Layout, Margin, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};
use std::{rc::Rc, time::Instant};
use tokio::sync::mpsc::{self, UnboundedReceiver};

const FOOTER_APP_VERSION: &str = concat!(" b4n v", env!("CARGO_PKG_VERSION"), " ");

/// Footer widget.
pub struct Footer {
    message: Option<Notification>,
    messages_rx: UnboundedReceiver<Notification>,
    message_received_time: Instant,
    icons: Vec<Icon>,
    icons_rx: UnboundedReceiver<IconAction>,
    footer_tx: NotificationSink,
}

impl Default for Footer {
    fn default() -> Self {
        let (messages_tx, messages_rx) = mpsc::unbounded_channel();
        let (icons_tx, icons_rx) = mpsc::unbounded_channel();
        let footer_tx = NotificationSink::new(messages_tx, icons_tx);

        Footer {
            message: None,
            messages_rx,
            message_received_time: Instant::now(),
            icons: Vec::new(),
            icons_rx,
            footer_tx,
        }
    }
}

impl Footer {
    pub fn transmitter(&self) -> &NotificationSink {
        &self.footer_tx
    }

    pub fn get_transmitter(&self) -> NotificationSink {
        self.footer_tx.clone()
    }

    /// Returns layout that can be used to draw [`Footer`].\
    /// **Note** that returned slice has two elements, the second one is for the footer itself.
    pub fn get_layout(area: Rect) -> Rc<[Rect]> {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Fill(1), Constraint::Length(1)])
            .split(area)
    }

    /// Draws [`Footer`] on the provided frame area.
    pub fn draw(&mut self, frame: &mut Frame<'_>, area: Rect, theme: &Theme) {
        self.draw_footer(frame, area, theme);

        if self.has_message_to_show()
            && let Some(message) = &self.message
        {
            let [area] = Layout::horizontal([Constraint::Length(message.text.chars().count() as u16)])
                .flex(Flex::Center)
                .areas(area.inner(Margin::new(2, 0)));
            frame.render_widget(Footer::get_message(&message.text, message.is_error, &theme.colors), area);
        }
    }

    fn draw_footer(&mut self, frame: &mut Frame<'_>, area: Rect, theme: &Theme) {
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![
                Constraint::Length(u16::try_from(FOOTER_APP_VERSION.len() + 2).unwrap_or_default()),
                Constraint::Fill(1),
                Constraint::Length(2),
            ])
            .split(area);

        self.update_current_icons();
        let colors = &theme.colors;

        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("", Style::new().fg(colors.footer.text.bg).bg(colors.text.bg)),
                Span::styled(" ", &colors.footer.text),
                Span::styled(FOOTER_APP_VERSION, &colors.footer.text),
            ])),
            layout[0],
        );

        if self.icons.is_empty() {
            frame.render_widget(Block::new().style(&colors.footer.text), layout[1]);
        } else {
            frame.render_widget(
                Paragraph::new(self.get_icons(layout[1].width.into(), &theme.colors)),
                layout[1],
            );
        }

        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(" ", &colors.footer.text),
                Span::styled("", Style::new().fg(colors.footer.text.bg).bg(colors.text.bg)),
            ])),
            layout[2],
        );
    }

    /// Returns formatted message to show.
    fn get_message<'a>(message: &'a str, is_error: bool, colors: &ThemeColors) -> Line<'a> {
        Line::styled(message, if is_error { &colors.footer.error } else { &colors.footer.info })
    }

    /// Returns `true` if there is a message to show.
    fn has_message_to_show(&mut self) -> bool {
        self.update_current_message();
        if let Some(message) = &self.message {
            if self.message_received_time.elapsed().as_millis() <= u128::from(message.duration) {
                true
            } else {
                self.message = None;
                false
            }
        } else {
            false
        }
    }

    /// Gets the last message from unbounded channel and sets it as active.
    fn update_current_message(&mut self) {
        let mut message = None;
        while let Ok(current) = self.messages_rx.try_recv() {
            message = Some(current);
        }

        if message.is_some() {
            self.message = message;
            self.message_received_time = Instant::now();
        }
    }

    /// Returns formatted icons to show.
    fn get_icons(&self, width: usize, colors: &ThemeColors) -> Line<'_> {
        let mut spans = Vec::with_capacity(self.icons.len());
        let mut total = 0;

        for icon in &self.icons {
            let color = match icon.kind {
                IconKind::Default => &colors.footer.text,
                IconKind::Success => &colors.footer.info,
                IconKind::Error => &colors.footer.error,
            };

            if let Some(icon) = icon.icon.as_ref() {
                spans.push(Span::styled(icon.to_string(), color));
                total += 1;
            }

            if let Some(text) = icon.text.as_deref() {
                spans.push(Span::styled(text, color));
                total += text.chars().count();
            }

            spans.push(Span::styled(" ", &colors.footer.text));
            total += 1;
        }

        spans.insert(0, Span::styled(" ".repeat(width.saturating_sub(total)), &colors.footer.text));
        Line::from(spans)
    }

    /// Updates all currently visible icons with the ones from the icons channel.
    fn update_current_icons(&mut self) {
        while let Ok(action) = self.icons_rx.try_recv() {
            match action {
                IconAction::Add(icon) => {
                    if let Some(index) = self.icons.iter().position(|i| i.id == icon.id) {
                        self.icons[index] = icon;
                    } else {
                        self.icons.push(icon);
                    }
                },
                IconAction::Remove(id) => self.icons.retain(|i| i.id != id),
            }
        }

        self.icons.sort_by_key(|i| i.id);
    }
}
