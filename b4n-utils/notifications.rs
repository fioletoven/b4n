use tokio::sync::mpsc::UnboundedSender;

const DEFAULT_MESSAGE_DURATION: u16 = 5_000;

/// Represents notification icon or text kind.
pub enum IconKind {
    Default,
    Success,
    Error,
}

/// Defines possible actions for managing notification icons.
pub enum IconAction {
    Add(Icon),
    Remove(&'static str),
}

/// Notification icon to show.
pub struct Icon {
    pub id: &'static str,
    pub icon: Option<char>,
    pub text: Option<String>,
    pub kind: IconKind,
}

impl Icon {
    /// Creates new [`Icon`] instance.
    fn new(id: &'static str) -> Self {
        Self {
            id,
            icon: None,
            text: None,
            kind: IconKind::Default,
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

    /// Sets kind.
    fn with_kind(mut self, kind: IconKind) -> Self {
        self.kind = kind;
        self
    }
}

/// Message notification to show.
pub struct Notification {
    pub text: String,
    pub is_error: bool,
    pub duration: u16,
}

impl Notification {
    /// Creates new [`Notification`] instance.
    fn new(text: String, is_error: bool, duration: u16) -> Self {
        Self {
            text,
            is_error,
            duration: if duration == 0 { DEFAULT_MESSAGE_DURATION } else { duration },
        }
    }
}

/// Notifications sink for messages and icons.
#[derive(Debug, Clone)]
pub struct NotificationSink {
    messages_tx: UnboundedSender<Notification>,
    icons_tx: UnboundedSender<IconAction>,
}

impl NotificationSink {
    /// Creates new [`NotificationSink`] instance.
    pub fn new(messages_tx: UnboundedSender<Notification>, icons_tx: UnboundedSender<IconAction>) -> Self {
        Self { messages_tx, icons_tx }
    }

    /// Displays an informational message for the specified duration (in milliseconds).
    pub fn show_info(&self, text: impl Into<String>, duration: u16) {
        let _ = self.messages_tx.send(Notification::new(text.into(), false, duration));
    }

    /// Displays an error message for the specified duration (in milliseconds).
    pub fn show_error(&self, text: impl Into<String>, duration: u16) {
        let _ = self.messages_tx.send(Notification::new(text.into(), true, duration));
    }

    /// Adds, updates, or removes an icon in the sink by its `id`.
    pub fn set_icon(&self, id: &'static str, icon: Option<char>, kind: IconKind) {
        let action = if let Some(icon) = icon {
            IconAction::Add(Icon::new(id).with_icon(icon).with_kind(kind))
        } else {
            IconAction::Remove(id)
        };
        let _ = self.icons_tx.send(action);
    }

    /// Adds, updates, or removes a text label in the sink by its `id`.
    pub fn set_text(&self, id: &'static str, text: Option<impl Into<String>>, kind: IconKind) {
        let action = if let Some(text) = text {
            IconAction::Add(Icon::new(id).with_text(text).with_kind(kind))
        } else {
            IconAction::Remove(id)
        };
        let _ = self.icons_tx.send(action);
    }

    /// Removes an icon or a text label from the sink by its `id`.
    pub fn reset(&self, id: &'static str) {
        let _ = self.icons_tx.send(IconAction::Remove(id));
    }
}
