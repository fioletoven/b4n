use ratatui::{
    layout::{Constraint, Direction, Flex, Layout, Margin, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Paragraph},
};
use std::{rc::Rc, time::Instant};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

use crate::core::SharedAppData;

const FOOTER_APP_VERSION: &str = concat!(" ", env!("CARGO_CRATE_NAME"), " v", env!("CARGO_PKG_VERSION"), " ");
const DEFAULT_MESSAGE_DURATION: u16 = 5_000;

/// Footer transmitter for messages and icons.
#[derive(Debug, Clone)]
pub struct FooterTx {
    messages_tx: UnboundedSender<FooterMessage>,
    icons_tx: UnboundedSender<FooterIconAction>,
}

impl FooterTx {
    /// Displays an informational message in the footer for the specified duration (in milliseconds).
    pub fn show_info(&self, text: impl Into<String>, duration: u16) {
        let _ = self.messages_tx.send(FooterMessage::new(text.into(), false, duration));
    }

    /// Displays an error message in the footer for the specified duration (in milliseconds).
    pub fn show_error(&self, text: impl Into<String>, duration: u16) {
        let _ = self.messages_tx.send(FooterMessage::new(text.into(), true, duration));
    }

    /// Adds, updates, or removes an icon in the footer by its `id`.
    ///
    /// Pass `Some(icon)` to add or update, or `None` to remove the icon.
    pub fn set_icon(&self, id: &'static str, icon: Option<char>) {
        let action = if let Some(icon) = icon {
            FooterIconAction::Add(FooterIcon::new(id).with_icon(icon))
        } else {
            FooterIconAction::Remove(id)
        };
        let _ = self.icons_tx.send(action);
    }

    /// Adds, updates, or removes a text label in the footer by its `id`.
    ///
    /// Pass `Some(text)` to add or update, or `None` to remove the text label.
    pub fn set_text(&self, id: &'static str, text: Option<impl Into<String>>) {
        let action = if let Some(text) = text {
            FooterIconAction::Add(FooterIcon::new(id).with_text(text))
        } else {
            FooterIconAction::Remove(id)
        };
        let _ = self.icons_tx.send(action);
    }
}

/// Footer widget.
pub struct Footer {
    app_data: SharedAppData,
    message: Option<FooterMessage>,
    messages_rx: UnboundedReceiver<FooterMessage>,
    message_received_time: Instant,
    icons: Vec<FooterIcon>,
    icons_rx: UnboundedReceiver<FooterIconAction>,
    footer_tx: FooterTx,
}

impl Footer {
    /// Creates new UI footer pane.
    pub fn new(app_data: SharedAppData) -> Self {
        let (messages_tx, messages_rx) = mpsc::unbounded_channel();
        let (icons_tx, icons_rx) = mpsc::unbounded_channel();
        let footer_tx = FooterTx { messages_tx, icons_tx };

        Footer {
            app_data,
            message: None,
            messages_rx,
            message_received_time: Instant::now(),
            icons: Vec::new(),
            icons_rx,
            footer_tx,
        }
    }

    pub fn transmitter(&self) -> &FooterTx {
        &self.footer_tx
    }

    pub fn get_transmitter(&self) -> FooterTx {
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
    pub fn draw(&mut self, frame: &mut ratatui::Frame<'_>, area: Rect) {
        self.draw_footer(frame, area);

        if self.has_message_to_show() {
            if let Some(message) = &self.message {
                let [area] = Layout::horizontal([Constraint::Length(message.text.chars().count() as u16)])
                    .flex(Flex::Center)
                    .areas(area.inner(Margin::new(2, 0)));
                frame.render_widget(self.get_message(&message.text, message.is_error), area);
            }
        }
    }

    fn draw_footer(&mut self, frame: &mut ratatui::Frame<'_>, area: Rect) {
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![
                Constraint::Length(u16::try_from(FOOTER_APP_VERSION.len() + 2).unwrap_or_default()),
                Constraint::Fill(1),
                Constraint::Length(2),
            ])
            .split(area);

        self.update_current_icons();
        let colors = &self.app_data.borrow().theme.colors;

        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("", Style::new().fg(colors.footer.text.bg)),
                Span::styled(" ", &colors.footer.text),
                Span::styled(FOOTER_APP_VERSION, &colors.footer.text),
            ])),
            layout[0],
        );

        if self.icons.is_empty() {
            frame.render_widget(Block::new().style(&colors.footer.text), layout[1]);
        } else {
            frame.render_widget(Paragraph::new(self.get_icons(layout[1].width.into())), layout[1]);
        }

        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(" ", &colors.footer.text),
                Span::styled("", Style::new().fg(colors.footer.text.bg)),
            ])),
            layout[2],
        );
    }

    /// Returns formatted message to show.
    fn get_message<'a>(&self, message: &'a str, is_error: bool) -> Line<'a> {
        let colors = &self.app_data.borrow().theme.colors;
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
    fn get_icons(&self, width: usize) -> Line<'_> {
        let mut icons = String::new();
        for icon in &self.icons {
            if let Some(icon) = icon.icon.as_ref() {
                icons.push(*icon);
            }
            if let Some(text) = icon.text.as_deref() {
                icons.push_str(text);
                icons.push(' ');
            }
        }

        if !icons.ends_with(' ') {
            icons.push(' ');
        }

        let colors = &self.app_data.borrow().theme.colors;
        Line::from(Span::styled(format!("{icons:>width$}"), &colors.footer.text))
    }

    /// Updates all currently visible icons with the ones from the icons channel.
    fn update_current_icons(&mut self) {
        while let Ok(action) = self.icons_rx.try_recv() {
            match action {
                FooterIconAction::Add(icon) => {
                    if let Some(index) = self.icons.iter().position(|i| i.id == icon.id) {
                        self.icons[index] = icon;
                    } else {
                        self.icons.push(icon);
                    }
                },
                FooterIconAction::Remove(id) => self.icons.retain(|i| i.id != id),
            }
        }
    }
}

/// Footer message to show.
struct FooterMessage {
    text: String,
    is_error: bool,
    duration: u16,
}

impl FooterMessage {
    /// Creates new [`FooterMessage`] instance.
    fn new(text: String, is_error: bool, duration: u16) -> Self {
        Self {
            text,
            is_error,
            duration: if duration == 0 { DEFAULT_MESSAGE_DURATION } else { duration },
        }
    }
}

/// Defines possible actions for managing Footer icons.
enum FooterIconAction {
    Add(FooterIcon),
    Remove(&'static str),
}

/// Footer icon to show.
struct FooterIcon {
    id: &'static str,
    icon: Option<char>,
    text: Option<String>,
}

impl FooterIcon {
    /// Creates new [`FooterIcon`] instance.
    fn new(id: &'static str) -> Self {
        Self {
            id,
            icon: None,
            text: None,
        }
    }

    /// Adds icon.
    fn with_icon(mut self, icon: char) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Adds text.
    fn with_text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }
}
