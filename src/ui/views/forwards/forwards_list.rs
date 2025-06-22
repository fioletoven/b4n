use delegate::delegate;
use std::{collections::HashMap, rc::Rc};

use crate::ui::{
    ResponseEvent, Responsive, Table, ViewType,
    colors::TextColors,
    lists::{BasicFilterContext, Column, FilterableList, Header, Item, NAMESPACE, TabularList},
    theme::Theme,
};

use super::PortForwardItem;

/// Port forward tasks list.
pub struct PortForwardsList {
    pub table: TabularList<PortForwardItem, BasicFilterContext>,
    width: usize,
}

impl Default for PortForwardsList {
    fn default() -> Self {
        Self {
            table: TabularList {
                header: header(),
                ..Default::default()
            },
            width: 0,
        }
    }
}

impl PortForwardsList {
    /// Updates [`PortForwardsList`] with new data from [`Vec<PortForwardItem>`].
    pub fn update(&mut self, items: Vec<PortForwardItem>) {
        let (sort_by, is_descending) = self.table.header.sort_info();
        self.table.list.items = Some(FilterableList::from(items.into_iter().map(Item::new).collect()));
        self.table.sort(sort_by, is_descending);
    }
}

impl Responsive for PortForwardsList {
    fn process_key(&mut self, key: crossterm::event::KeyEvent) -> ResponseEvent {
        self.table.process_key(key)
    }
}

impl Table for PortForwardsList {
    delegate! {
        to self.table.list {
            fn clear(&mut self);
            fn len(&self) -> usize;
            fn is_filtered(&self) -> bool;
            fn filter(&mut self, filter: Option<String>);
            fn get_filter(&self) -> Option<&str>;
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

    fn sort(&mut self, column_no: usize, is_descending: bool) {
        self.table.sort(column_no, is_descending);
    }

    fn get_sort_symbols(&self) -> Rc<[char]> {
        self.table.header.get_sort_symbols()
    }

    /// Returns items from the current page in a form of text lines to display and colors for that lines.
    fn get_paged_items(&self, theme: &Theme, view: ViewType, width: usize) -> Option<Vec<(String, TextColors)>> {
        if let Some(list) = self.table.list.get_page() {
            let (namespace_width, name_width, name_extra_width) = self.table.header.get_widths(view, width);

            let mut result = Vec::with_capacity(self.table.list.page_height.into());
            for item in list {
                result.push((
                    item.data.get_text(
                        view,
                        &self.table.header,
                        width,
                        namespace_width,
                        name_width + name_extra_width,
                    ),
                    item.data.get_colors(theme, item.is_active, item.is_selected),
                ));
            }

            return Some(result);
        }

        None
    }

    fn get_header(&mut self, view: ViewType, width: usize) -> &str {
        tracing::info!("get header text for view: {:?}", view);
        self.table.header.get_text(view, width)
    }
}

/// Returns [`Header`] for the port forward task.
pub fn header() -> Header {
    Header::from(
        NAMESPACE.clone(),
        Some(Box::new([
            Column::bound("LOCAL", 14, 22, false),
            Column::fixed("REMOTE", 8, false),
            Column::fixed("ACTIVE", 8, true),
            Column::fixed("OVERALL", 8, true),
            Column::fixed("ERROR", 8, true),
        ])),
        Rc::new([' ', 'N', 'L', 'R', 'C', 'O', 'E', 'A']),
    )
}
