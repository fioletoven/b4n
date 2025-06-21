use delegate::delegate;
use std::collections::HashMap;

use crate::ui::{
    ResponseEvent, Responsive, Table, ViewType,
    colors::TextColors,
    lists::{BasicFilterContext, FilterableList, Header, Item, ScrollableList},
    theme::Theme,
};

use super::PortForwardItem;

/// Port forward tasks list.
#[derive(Default)]
pub struct PortForwardsList {
    pub list: ScrollableList<PortForwardItem, BasicFilterContext>,
    header: Header,
    header_cache: String,
    width: usize,
}

impl PortForwardsList {
    /// Updates [`PortForwardsList`] with new data from [`Vec<PortForwardItem>`].
    pub fn update(&mut self, items: Vec<PortForwardItem>, sort_by: usize, is_descending: bool) {
        self.list.items = Some(FilterableList::from(items.into_iter().map(Item::new).collect()));
        self.list.sort(sort_by, is_descending);
    }
}

impl Responsive for PortForwardsList {
    fn process_key(&mut self, key: crossterm::event::KeyEvent) -> ResponseEvent {
        self.list.process_key(key)
    }
}

impl Table for PortForwardsList {
    delegate! {
        to self.list {
            fn clear(&mut self);
            fn len(&self) -> usize;
            fn is_filtered(&self) -> bool;
            fn filter(&mut self, filter: Option<String>);
            fn get_filter(&self) -> Option<&str>;
            fn sort(&mut self, column_no: usize, is_descending: bool);
            fn get_highlighted_item_index(&self) -> Option<usize>;
            fn get_highlighted_item_name(&self) -> Option<&str>;
            fn highlight_item_by_name(&mut self, name: &str) -> bool;
            fn highlight_item_by_name_start(&mut self, text: &str) -> bool;
            fn highlight_first_item(&mut self) -> bool;
            fn deselect_all(&mut self);
            fn invert_selection(&mut self);
            fn select_highlighted_item(&mut self);
            fn get_selected_items(&self) -> HashMap<&str, Vec<&str>>;
            fn is_anything_selected(&self) -> bool;
            fn update_page(&mut self, new_height: u16);
            fn get_paged_names(&self, width: usize) -> Option<Vec<(String, bool)>>;
        }
    }

    /// Returns items from the current page in a form of text lines to display and colors for that lines.
    fn get_paged_items(&self, theme: &Theme, view: ViewType, width: usize) -> Option<Vec<(String, TextColors)>> {
        if let Some(list) = self.list.get_page() {
            let (namespace_width, name_width, name_extra_width) = self.header.get_widths(view, width);

            let mut result = Vec::with_capacity(self.list.page_height.into());
            for item in list {
                result.push((
                    item.data
                        .get_text(view, &self.header, width, namespace_width, name_width + name_extra_width),
                    item.data.get_colors(theme, item.is_active, item.is_selected),
                ));
            }

            return Some(result);
        }

        None
    }

    fn get_header(&mut self, view: ViewType, width: usize) -> &str {
        let (namespace_width, name_width, _) = self.header.get_widths(view, width);
        self.header_cache = self.header.get_text(view, namespace_width, name_width, width);

        &self.header_cache
    }
}
