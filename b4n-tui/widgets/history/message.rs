use b4n_common::{Notification, add_padding, truncate};
use b4n_config::themes::{TextColors, Theme};
use b4n_list::{BasicFilterContext, Filterable, Row};
use std::borrow::Cow;
use std::time::Instant;

/// Footer history message item.
pub struct MessageItem {
    pub uid: String,
    pub group: &'static str,
    pub message: String,
    time: Instant,
    is_error: bool,
}

impl MessageItem {
    /// Creates new [`MessageItem`] instance from the [`Notification`] and it's time.
    pub fn from(notification: &Notification, time: Instant, id: usize) -> Self {
        Self {
            uid: format!("_{id}_"),
            group: "notification",
            message: notification.text.clone(),
            time,
            is_error: notification.is_error,
        }
    }

    /// Returns text that can be displayed as a list line.
    pub fn get_text(&self, width: usize) -> String {
        let time = format_elapsed(self.time);
        let width = width.saturating_sub(9);
        format!("{:<width$}  {time:>7}", truncate(&self.message, width))
    }

    /// Returns color for the message item.
    pub fn get_color(&self, theme: &Theme, is_active: bool) -> TextColors {
        if is_active {
            if self.is_error {
                theme.colors.footer.error.to_reverted()
            } else {
                theme.colors.footer.info.to_reverted()
            }
        } else if self.is_error {
            theme.colors.footer.error
        } else {
            theme.colors.footer.info
        }
    }
}

impl Row for MessageItem {
    fn uid(&self) -> &str {
        &self.uid
    }

    fn group(&self) -> &str {
        self.group
    }

    fn name(&self) -> &str {
        &self.message
    }

    fn get_name(&self, width: usize) -> String {
        add_padding(&self.message, width)
    }

    fn column_text(&self, column: usize) -> Cow<'_, str> {
        Cow::Borrowed(match column {
            0 => self.group,
            1 => &self.message,
            _ => "n/a",
        })
    }

    fn column_sort_text(&self, column: usize) -> &str {
        match column {
            0 => self.group,
            1 => &self.message,
            _ => "n/a",
        }
    }
}

impl Filterable<BasicFilterContext> for MessageItem {
    fn get_context(pattern: &str, _: Option<&str>) -> BasicFilterContext {
        pattern.to_owned().into()
    }

    fn is_matching(&self, context: &mut BasicFilterContext) -> bool {
        self.contains(&context.pattern)
    }
}

pub fn format_elapsed(start: Instant) -> String {
    let total_secs = start.elapsed().as_secs();
    let days = total_secs / 86_400;
    let hours = (total_secs / 3_600) % 24;

    if days > 0 {
        format!("{days}d{hours:0>2}h")
    } else {
        let minutes = (total_secs / 60) % 60;
        if hours > 0 {
            format!("{hours}h{minutes:0>2}m")
        } else {
            let secs = total_secs % 60;
            if minutes > 0 {
                format!("{minutes}m{secs:0>2}s")
            } else {
                format!("{secs}s")
            }
        }
    }
}
