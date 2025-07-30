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

/// Footer message to show.
pub struct FooterMessage {
    pub text: String,
    pub is_error: bool,
    pub duration: u16,
}

impl FooterMessage {
    /// Creates new info [`FooterMessage`] instance.
    pub fn info(text: impl Into<String>, duration: u16) -> Self {
        Self {
            text: text.into(),
            is_error: false,
            duration: if duration == 0 { DEFAULT_MESSAGE_DURATION } else { duration },
        }
    }

    /// Creates new error [`FooterMessage`] instance.
    pub fn error(text: impl Into<String>, duration: u16) -> Self {
        Self {
            text: text.into(),
            is_error: true,
            duration: if duration == 0 { DEFAULT_MESSAGE_DURATION } else { duration },
        }
    }
}

/// Defines possible actions for managing Footer icons.
pub enum FooterIconAction {
    Add,
    Remove,
}

/// Footer icon to show.
pub struct FooterIcon {
    pub id: &'static str,
    pub icon: Option<char>,
    pub text: Option<String>,
}

impl FooterIcon {
    /// Creates new [`FooterIcon`] instance.
    pub fn new(id: &'static str, icon: char, text: String) -> Self {
        Self {
            id,
            icon: Some(icon),
            text: Some(text),
        }
    }

    /// Creates new empty [`FooterIcon`] instance.
    pub fn empty(id: &'static str) -> Self {
        Self {
            id,
            icon: None,
            text: None,
        }
    }

    /// Creates new icon [`FooterIcon`] instance.
    pub fn icon(id: &'static str, icon: char) -> Self {
        Self {
            id,
            icon: Some(icon),
            text: None,
        }
    }

    /// Creates new text [`FooterIcon`] instance.
    pub fn text(id: &'static str, text: String) -> Self {
        Self {
            id,
            icon: None,
            text: Some(text),
        }
    }
}

/// Footer widget.
pub struct Footer {
    app_data: SharedAppData,
    message: Option<FooterMessage>,
    messages_tx: UnboundedSender<FooterMessage>,
    messages_rx: UnboundedReceiver<FooterMessage>,
    message_received_time: Instant,
    icons: Vec<FooterIcon>,
    icons_tx: UnboundedSender<(FooterIconAction, FooterIcon)>,
    icons_rx: UnboundedReceiver<(FooterIconAction, FooterIcon)>,
}

impl Footer {
    /// Creates new UI footer pane.
    pub fn new(app_data: SharedAppData) -> Self {
        let (messages_tx, messages_rx) = mpsc::unbounded_channel();
        let (icons_tx, icons_rx) = mpsc::unbounded_channel();

        Footer {
            app_data,
            message: None,
            messages_tx,
            messages_rx,
            message_received_time: Instant::now(),
            icons: Vec::new(),
            icons_tx,
            icons_rx,
        }
    }

    /// Returns [`FooterMessage`]s unbounded sender.
    pub fn get_messages_sender(&self) -> UnboundedSender<FooterMessage> {
        self.messages_tx.clone()
    }

    /// Sends [`FooterMessage`] at the end of the queue.
    pub fn send_message(&mut self, message: FooterMessage) {
        self.messages_tx.send(message).unwrap();
    }

    /// Returns [`FooterIcon`]s unbounded sender.
    pub fn get_icons_sender(&self) -> UnboundedSender<(FooterIconAction, FooterIcon)> {
        self.icons_tx.clone()
    }

    /// Sends [`FooterIcon`] at the end of the queue.
    pub fn send_icon(&mut self, action: FooterIconAction, icon: FooterIcon) {
        self.icons_tx.send((action, icon)).unwrap();
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
        while let Ok(current) = self.icons_rx.try_recv() {
            match current.0 {
                FooterIconAction::Add => {
                    if let Some(index) = self.icons.iter().position(|i| i.id == current.1.id) {
                        self.icons[index] = current.1;
                    } else {
                        self.icons.push(current.1);
                    }
                },
                FooterIconAction::Remove => self.icons.retain(|i| i.id != current.1.id),
            }
        }
    }
}
