use b4n_config::themes::{TextColors, Theme};
use b4n_kube::ContainerRef;
use b4n_list::Item;
use b4n_tui::table::{Column, Header, ItemExt, NAMESPACE, TabularList, ViewType};
use b4n_tui::{ResponseEvent, Responsive, TuiEvent, table::Table};
use delegate::delegate;
use std::{collections::HashMap, rc::Rc};

use crate::ui::views::PortForwardItem;
use crate::ui::views::forwards::PortForwardFilterContext;

/// Port forward tasks list.
pub struct PortForwardsList {
    pub table: TabularList<PortForwardItem, PortForwardFilterContext>,
}

impl Default for PortForwardsList {
    fn default() -> Self {
        Self {
            table: TabularList::new(header()),
        }
    }
}

impl PortForwardsList {
    /// Updates [`PortForwardsList`] with new data from [`Vec<PortForwardItem>`].
    pub fn update(&mut self, items: Vec<PortForwardItem>) {
        let (sort_by, is_descending) = self.table.header.sort_info();
        let highlighted_uid = self.table.list.get_highlighted_item_uid().map(String::from);
        let selected_uids: Vec<_> = self.table.list.get_selected_uids().iter().map(|&u| u.to_owned()).collect();
        let filter = self.table.list.filter().map(String::from);

        self.table.list.set_items(items.into_iter().map(Item::new).collect());
        self.table.sort(sort_by, is_descending);
        self.table.update_data_lengths();
        self.table.list.set_filter(filter);

        self.table.list.select_uids(selected_uids.as_slice());
        if let Some(uid) = highlighted_uid {
            self.table.list.highlight_item_by_uid(&uid);
        }
    }

    /// Copies filtered `self` into a new `Vec` of `ContainerRef`.
    pub fn to_container_vec(&self) -> Vec<ContainerRef> {
        self.table.list.iter().map(|i| (&i.data).into()).collect()
    }
}

impl Responsive for PortForwardsList {
    fn process_event(&mut self, event: &TuiEvent) -> ResponseEvent {
        self.table.process_event(event)
    }
}

impl Table for PortForwardsList {
    delegate! {
        to self.table.list {
            fn clear(&mut self);
            fn len(&self) -> usize;
            fn is_filtered(&self) -> bool;
            fn set_filter(&mut self, filter: Option<String>);
            fn filter(&self) -> Option<&str>;
            fn is_anything_highlighted(&self) -> bool;
            fn get_highlighted_item_index(&self) -> Option<usize>;
            fn get_highlighted_item_name(&self) -> Option<&str>;
            fn get_highlighted_item_uid(&self) -> Option<&str>;
            fn get_highlighted_item_line_no(&self) -> Option<u16>;
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
            fn get_paged_names(&self, width: usize) -> Vec<(String, bool)>;
        }
    }

    fn get_column_at_position(&self, position: usize) -> Option<usize> {
        self.table.get_column_at_position(position)
    }

    fn sort(&mut self, column_no: usize, is_descending: bool) {
        self.table.sort(column_no, is_descending);
    }

    fn toggle_sort(&mut self, column_no: usize) {
        self.table.toggle_sort(column_no);
    }

    fn get_sort_symbols(&self) -> Rc<[char]> {
        self.table.header.get_sort_symbols()
    }

    /// Returns items from the current page in a form of text lines to display and colors for that lines.
    fn get_paged_items(&self, theme: &Theme, view: ViewType, width: usize) -> Vec<(String, TextColors)> {
        let (namespace_width, name_width, name_extra_width) = self.table.header.get_widths(view, width);

        let mut result = Vec::with_capacity(self.table.list.page_height().into());
        for item in self.table.list.get_page() {
            result.push((
                item.get_text(
                    view,
                    &self.table.header,
                    width,
                    namespace_width,
                    name_width + name_extra_width,
                    self.table.offset(),
                ),
                item.data.get_colors(theme, item.is_active, item.is_selected),
            ));
        }

        result
    }

    fn get_header(&mut self, view: ViewType, width: usize) -> &str {
        self.table.header.get_text(view, width)
    }

    fn refresh_header(&mut self, view: ViewType, width: usize) {
        self.table.header.refresh_text(view, width);
    }

    fn offset(&self) -> usize {
        self.table.offset()
    }

    fn refresh_offset(&mut self) -> usize {
        self.table.get_offset()
    }
}

/// Returns [`Header`] for the port forward task.
pub fn header() -> Header {
    Header::from(
        NAMESPACE,
        Some(Box::new([
            Column::bound("LOCAL", 14, 22, false),
            Column::fixed("REMOTE", 8, true),
            Column::fixed("ACTIVE", 8, true),
            Column::fixed("ERRORS", 8, true),
            Column::fixed("TOTAL", 8, true),
        ])),
        Rc::new([' ', 'N', 'L', 'R', 'C', 'E', 'T', 'A']),
    )
}
