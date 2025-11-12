use b4n_tui::{ResponseEvent, TuiEvent};
use ratatui::{layout::Rect, widgets::Widget};

use crate::ui::presentation::{Content, content::search::PagePosition};

/// Context for the selected text.
#[derive(Default)]
pub struct SelectContext {
    start: Option<PagePosition>,
    end: Option<PagePosition>,
}

impl SelectContext {
    /// Process UI key/mouse event.
    pub fn process_event<T: Content>(
        &mut self,
        event: &TuiEvent,
        content: &mut T,
        position: PagePosition,
        area: Rect,
    ) -> ResponseEvent {
        ResponseEvent::NotHandled
    }
}

/// Widget that draws selection on the content.
pub struct ContentSelectWidget<'a> {
    pub context: &'a SelectContext,
    pub page_start: &'a PagePosition,
}

impl<'a> ContentSelectWidget<'a> {
    /// Creates new [`ContentSelectWidget`] instance.
    pub fn new(context: &'a SelectContext, page_start: &'a PagePosition) -> Self {
        Self { context, page_start }
    }
}

impl Widget for ContentSelectWidget<'_> {
    fn render(self, area: Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
    }
}
