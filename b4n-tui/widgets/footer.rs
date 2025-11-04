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
    trail: Vec<String>,
    trail_rx: UnboundedReceiver<Vec<String>>,
    show_trail: bool,
    message: Option<Notification>,
    messages_rx: UnboundedReceiver<Notification>,
    message_received_time: Instant,
    icons: Vec<Icon>,
    icons_rx: UnboundedReceiver<IconAction>,
    notifications_tx: NotificationSink,
}

impl Default for Footer {
    fn default() -> Self {
        let (messages_tx, messages_rx) = mpsc::unbounded_channel();
        let (icons_tx, icons_rx) = mpsc::unbounded_channel();
        let (trail_tx, trail_rx) = mpsc::unbounded_channel();
        let notifications_tx = NotificationSink::new(messages_tx, icons_tx, trail_tx);

        Footer {
            trail: Vec::new(),
            trail_rx,
            show_trail: true,
            message: None,
            messages_rx,
            message_received_time: Instant::now(),
            icons: Vec::new(),
            icons_rx,
            notifications_tx,
        }
    }
}

impl Footer {
    /// Returns a reference to the footer's [`NotificationSink`].
    pub fn transmitter(&self) -> &NotificationSink {
        &self.notifications_tx
    }

    /// Returns the footer's [`NotificationSink`].
    pub fn get_transmitter(&self) -> NotificationSink {
        self.notifications_tx.clone()
    }

    /// Sets whether to show the breadcrumb trail.
    pub fn show_breadcrumb_trail(&mut self, show: bool) {
        self.show_trail = show;
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
        self.update_current_icons();
        self.update_current_trail();

        let colors = &theme.colors;
        let (icons, icons_len) = self.get_icons(colors);
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![
                Constraint::Fill(1),
                Constraint::Length(1),
                Constraint::Length(u16::try_from(icons_len).unwrap_or_default()),
                Constraint::Length(2),
            ])
            .split(area);

        frame.render_widget(Paragraph::new(self.get_left_text(layout[0].width, colors)), layout[0]);
        frame.render_widget(Block::new().style(&colors.footer.text), layout[1]);
        frame.render_widget(Paragraph::new(icons), layout[2]);
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(" ", &colors.footer.text),
                Span::styled("", Style::new().fg(colors.footer.text.bg).bg(colors.text.bg)),
            ])),
            layout[3],
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

    /// Returns formatted icons to show.
    fn get_icons(&self, colors: &ThemeColors) -> (Line<'_>, usize) {
        if self.icons.is_empty() {
            return (Line::default(), 0);
        }

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

        (Line::from(spans), total)
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

    /// Gets the last breadcrumb trail from the unbounded channel.
    fn update_current_trail(&mut self) {
        while let Ok(trail) = self.trail_rx.try_recv() {
            self.trail = trail;
        }
    }

    /// Renders left text: app version or breadcrumb trail if one is available.
    fn get_left_text(&self, width: u16, colors: &ThemeColors) -> Line<'_> {
        let width = usize::from(width);
        let mut rendered = 0;
        let mut spans = Vec::with_capacity(10);
        let mut total = FOOTER_APP_VERSION.len();

        spans.push(Span::styled("", Style::new().fg(colors.footer.text.bg).bg(colors.text.bg)));
        spans.push(Span::styled(" ", &colors.footer.text));
        spans.push(Span::styled(FOOTER_APP_VERSION, &colors.footer.text));

        if self.show_trail && !self.trail.is_empty() {
            spans.push(Span::styled("  ", &colors.footer.text));
            total += 2;

            let separator_style = Style::new().fg(colors.footer.trail.dim).bg(colors.footer.trail.bg);
            for (i, element) in self.trail.iter().enumerate() {
                if i != 0 {
                    spans.push(Span::styled("  ", separator_style));
                    total += 3;
                }

                rendered = i;
                spans.push(Span::styled(element, &colors.footer.trail));
                total += element.chars().count();

                if total >= width {
                    break;
                }
            }

            if rendered + 1 == self.trail.len()
                && let Some(span) = spans.last_mut()
            {
                span.style = (&colors.footer.text).into();
            }
        }

        spans.push(Span::styled(" ".repeat(width.saturating_sub(total)), &colors.footer.text));
        Line::from(spans)
    }
}
