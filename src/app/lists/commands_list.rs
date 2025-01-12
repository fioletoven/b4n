use std::collections::HashMap;

use delegate::delegate;

use crate::{
    kubernetes::resources::Kind,
    ui::{colors::TextColors, theme::Theme, ResponseEvent, Responsive, Table, ViewType},
};

use super::{Command, Row, ScrollableList};

/// Commands list.
#[derive(Default)]
pub struct CommandsList {
    pub list: ScrollableList<Command>,
}

impl CommandsList {
    /// Creates new [`CommandList`] instance with the predefined list of commands.
    pub fn new(mut commands: Vec<Command>) -> Self {
        commands.push(Command::new("command".to_owned(), "quit".to_owned(), None));
        commands.push(Command::new(
            "command".to_owned(),
            "context".to_owned(),
            Some(vec!["ctx".to_owned()]),
        ));

        let mut list = ScrollableList::from(commands);
        list.sort(1, false);

        Self { list }
    }

    /// Creates new [`CommandsList`] instance that will include provided kinds and predefined commands.
    pub fn from(kinds: &ScrollableList<Kind>) -> Self {
        if let Some(items) = &kinds.items {
            CommandsList::new(
                items
                    .full_iter()
                    .map(|i| Command::new("resource".to_owned(), i.data.name().to_owned(), None))
                    .collect::<Vec<Command>>(),
            )
        } else {
            CommandsList::new(vec![])
        }
    }
}

impl Responsive for CommandsList {
    fn process_key(&mut self, key: crossterm::event::KeyEvent) -> ResponseEvent {
        self.list.process_key(key)
    }
}

impl Table for CommandsList {
    delegate! {
        to self.list {
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
    /// As commands are used only in selector, we don't care to implement this.
    fn get_paged_items(&self, _theme: &Theme, _view: ViewType, _width: usize) -> Option<Vec<(String, TextColors)>> {
        None
    }

    fn get_header(&self, _view: ViewType, width: usize) -> String {
        format!("{1:<0$}", width, "KIND")
    }
}