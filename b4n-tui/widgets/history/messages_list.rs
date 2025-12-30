use b4n_config::themes::{TextColors, Theme};
use b4n_list::{BasicFilterContext, ScrollableList};
use delegate::delegate;
use std::collections::HashMap;

use crate::table::{Table, ViewType};
use crate::widgets::history::MessageItem;
use crate::{ResponseEvent, Responsive, TuiEvent};

#[derive(Default)]
pub struct MessagesList {
    pub list: ScrollableList<MessageItem, BasicFilterContext>,
}

impl From<Vec<MessageItem>> for MessagesList {
    fn from(value: Vec<MessageItem>) -> Self {
        Self {
            list: ScrollableList::from(value),
        }
    }
}

impl MessagesList {
    pub fn update(&mut self, new_list: Vec<MessageItem>) {
        let uid = self.list.get_highlighted_item_uid().map(String::from);
        self.list = ScrollableList::from(new_list);
        if let Some(uid) = uid {
            self.list.highlight_item_by_uid(&uid);
        }
    }
}

impl Responsive for MessagesList {
    fn process_event(&mut self, event: &TuiEvent) -> ResponseEvent {
        self.list.process_event(event)
    }
}

impl Table for MessagesList {
    delegate! {
        to self.list {
            fn clear(&mut self);
            fn len(&self) -> usize;
            fn is_filtered(&self) -> bool;
            fn filter(&mut self, filter: Option<String>);
            fn get_filter(&self) -> Option<&str>;
            fn sort(&mut self, column_no: usize, is_descending: bool);
            fn is_anything_highlighted(&self) -> bool;
            fn get_highlighted_item_index(&self) -> Option<usize>;
            fn get_highlighted_item_name(&self) -> Option<&str>;
            fn get_highlighted_item_uid(&self) -> Option<&str>;
            fn highlight_item_by_name(&mut self, name: &str) -> bool;
            fn highlight_item_by_name_start(&mut self, text: &str) -> bool;
            fn highlight_item_by_uid(&mut self, uid: &str) -> bool;
            fn highlight_item_by_line(&mut self, line_no: u16) -> bool;
            fn highlight_first_item(&mut self) -> bool;
            fn unhighlight_item(&mut self);
            fn select_all(&mut self);
            fn deselect_all(&mut self);
            fn invert_selection(&mut self);
            fn select_highlighted_item(&mut self);
            fn get_selected_items(&self) -> HashMap<&str, Vec<&str>>;
            fn is_anything_selected(&self) -> bool;
            fn update_page(&mut self, new_height: u16);
            fn get_paged_names(&self, width: usize) -> Option<Vec<(String, bool)>>;
        }
    }

    /// Not implemented for [`MessagesList`].
    fn get_column_at_position(&self, _position: usize) -> Option<usize> {
        None
    }

    /// Not implemented for [`MessagesList`].
    fn toggle_sort(&mut self, _column_no: usize) {
        // pass
    }

    /// Returns items from the current page in a form of text lines to display and colors for that lines.
    fn get_paged_items(&self, theme: &Theme, _view: ViewType, width: usize) -> Option<Vec<(String, TextColors)>> {
        if let Some(list) = self.list.get_page() {
            let mut result = Vec::with_capacity(self.list.page_height.into());
            for item in list {
                result.push((item.data.get_text(width), item.data.get_color(theme, item.is_active)));
            }

            return Some(result);
        }

        None
    }
}
