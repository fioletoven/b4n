use ratatui::{
    layout::{Constraint, Direction, Flex, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
};
use std::{rc::Rc, time::Instant};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

use crate::app::SharedAppData;

/// Footer message to show.
pub struct FooterMessage {
    pub text: String,
    pub is_error: bool,
    pub duration: u16,
}

impl FooterMessage {
    /// Creates new info [`FooterMessage`] instance.
    pub fn info(text: String, duration: u16) -> Self {
        Self {
            text,
            is_error: false,
            duration,
        }
    }

    /// Creates new error [`FooterMessage`] instance.
    pub fn error(text: String, duration: u16) -> Self {
        Self {
            text,
            is_error: true,
            duration,
        }
    }
}

/// Footer widget.
pub struct Footer {
    app_data: SharedAppData,
    version: String,
    message: Option<FooterMessage>,
    messages_tx: UnboundedSender<FooterMessage>,
    messages_rx: UnboundedReceiver<FooterMessage>,
    message_received_time: Instant,
}

impl Footer {
    /// Creates new UI footer pane.
    pub fn new(app_data: SharedAppData) -> Self {
        let version = format!(" {} v{} ", env!("CARGO_CRATE_NAME"), env!("CARGO_PKG_VERSION"));
        let (messages_tx, messages_rx) = mpsc::unbounded_channel();

        Footer {
            app_data,
            version,
            message: None,
            messages_tx,
            messages_rx,
            message_received_time: Instant::now(),
        }
    }

    /// Returns [`FooterMessage`]s unbounded sender.
    pub fn get_messages_sender(&self) -> UnboundedSender<FooterMessage> {
        self.messages_tx.clone()
    }

    /// Returns layout that can be used to draw [`Footer`].  
    /// **Note** that returned slice has two elements, the second one is for the footer itself.
    pub fn get_layout(area: Rect) -> Rc<[Rect]> {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Fill(1), Constraint::Length(1)])
            .split(area)
    }

    /// Draws [`Footer`] on the provided frame area.
    pub fn draw(&mut self, frame: &mut ratatui::Frame<'_>, area: Rect) {
        let footer = self.get_footer(area.width.into());
        frame.render_widget(Paragraph::new(footer), area);

        if self.has_message_to_show() {
            if let Some(message) = &self.message {
                let [area] = Layout::horizontal([Constraint::Length(message.text.chars().count() as u16)])
                    .flex(Flex::Center)
                    .areas(area);
                frame.render_widget(self.get_message(&message.text, message.is_error), area);
            }
        }
    }

    /// Returns formatted footer line.
    fn get_footer(&self, terminal_width: usize) -> Line<'_> {
        let footer = format!(" {1:<0$}", terminal_width - 3, &self.version);
        let colors = &self.app_data.borrow().config.theme.colors;

        Line::from(vec![
            Span::styled("", Style::new().fg(colors.footer.text.bg)),
            Span::styled(footer, &colors.footer.text),
            Span::styled("", Style::new().fg(colors.footer.text.bg)),
        ])
    }

    /// Returns formatted message to show.
    fn get_message<'a>(&self, message: &'a str, is_error: bool) -> Line<'a> {
        let colors = &self.app_data.borrow().config.theme.colors;
        Line::styled(message, if is_error { &colors.footer.error } else { &colors.footer.info })
    }

    /// Returns `true` if there is a message to show.
    fn has_message_to_show(&mut self) -> bool {
        self.update_current_message();
        if let Some(message) = &self.message {
            if self.message_received_time.elapsed().as_millis() <= message.duration.into() {
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
}
