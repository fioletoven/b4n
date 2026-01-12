use tokio::sync::mpsc::UnboundedSender;

pub const DEFAULT_MESSAGE_DURATION: u16 = 5_000;
pub const DEFAULT_ERROR_DURATION: u16 = 10_000;

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

/// Type of the notification.
#[derive(Clone, PartialEq)]
pub enum NotificationKind {
    Info,
    Error,
    Hint,
}

/// Message notification to show.
#[derive(Clone)]
pub struct Notification {
    pub text: String,
    pub kind: NotificationKind,
    pub duration: u16,
}

impl Notification {
    /// Creates new [`Notification`] instance.
    fn new(text: String, kind: NotificationKind, duration: u16) -> Self {
        Self { text, kind, duration }
    }
}

/// Notifications sink for breadcrumb trail, messages and icons.
#[derive(Debug, Clone)]
pub struct NotificationSink {
    messages: UnboundedSender<Notification>,
    icons: UnboundedSender<IconAction>,
    trail: UnboundedSender<Vec<String>>,
}

impl NotificationSink {
    /// Creates new [`NotificationSink`] instance.
    pub fn new(
        messages_tx: UnboundedSender<Notification>,
        icons_tx: UnboundedSender<IconAction>,
        trail_tx: UnboundedSender<Vec<String>>,
    ) -> Self {
        Self {
            messages: messages_tx,
            icons: icons_tx,
            trail: trail_tx,
        }
    }

    /// Displays an informational message for the specified duration (in milliseconds).
    pub fn show_info(&self, text: impl Into<String>, duration: u16) {
        let _ = self
            .messages
            .send(Notification::new(text.into(), NotificationKind::Info, duration));
    }

    /// Displays an error message for the specified duration (in milliseconds).
    pub fn show_error(&self, text: impl Into<String>, duration: u16) {
        let _ = self
            .messages
            .send(Notification::new(text.into(), NotificationKind::Error, duration));
    }

    /// Starts displaying a hint message in the footer (if there is a space for it).
    pub fn show_hint(&self, text: impl Into<String>) {
        let _ = self.messages.send(Notification::new(text.into(), NotificationKind::Hint, 0));
    }

    /// Stops displaying a hint message if any is displayed.
    pub fn hide_hint(&self) {
        let _ = self
            .messages
            .send(Notification::new(String::new(), NotificationKind::Hint, 0));
    }

    /// Adds, updates, or removes an icon in the sink by its `id`.
    pub fn set_icon(&self, id: &'static str, icon: Option<char>, kind: IconKind) {
        let action = if let Some(icon) = icon {
            IconAction::Add(Icon::new(id).with_icon(icon).with_kind(kind))
        } else {
            IconAction::Remove(id)
        };
        let _ = self.icons.send(action);
    }

    /// Adds, updates, or removes a text label in the sink by its `id`.
    pub fn set_text(&self, id: &'static str, text: Option<impl Into<String>>, kind: IconKind) {
        let action = if let Some(text) = text {
            IconAction::Add(Icon::new(id).with_text(text).with_kind(kind))
        } else {
            IconAction::Remove(id)
        };
        let _ = self.icons.send(action);
    }

    /// Removes an icon or a text label from the sink by its `id`.
    pub fn reset(&self, id: &'static str) {
        let _ = self.icons.send(IconAction::Remove(id));
    }

    /// Sets breadcrumb trail data.
    pub fn set_breadcrumb_trail(&self, trail: Vec<String>) {
        let _ = self.trail.send(trail);
    }
}
