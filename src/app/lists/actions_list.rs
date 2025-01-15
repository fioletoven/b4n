use std::collections::HashMap;

use delegate::delegate;

use crate::{
    kubernetes::resources::Kind,
    ui::{colors::TextColors, theme::Theme, widgets::Action, ResponseEvent, Responsive, Table, ViewType},
};

use super::ScrollableList;

/// UI actions list.
#[derive(Default)]
pub struct ActionsList {
    pub list: ScrollableList<Action>,
}

impl ActionsList {
    /// Creates new [`ActionsList`] instance with the predefined list of actions.
    pub fn new(actions: Vec<Action>) -> Self {
        let mut list = ScrollableList::from(insert_predefined_actions(actions));
        list.sort(1, false);

        Self { list }
    }

    /// Creates new [`ActionsList`] instance that will include provided kinds and predefined actions.
    pub fn from(kinds: &ScrollableList<Kind>) -> Self {
        if let Some(items) = &kinds.items {
            ActionsList::new(items.full_iter().map(|i| Action::from(&i.data)).collect::<Vec<Action>>())
        } else {
            ActionsList::new(vec![])
        }
    }
}

impl Responsive for ActionsList {
    fn process_key(&mut self, key: crossterm::event::KeyEvent) -> ResponseEvent {
        self.list.process_key(key)
    }
}

impl Table for ActionsList {
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
    /// As actions are used only in selector, we don't care to implement this.
    fn get_paged_items(&self, _theme: &Theme, _view: ViewType, _width: usize) -> Option<Vec<(String, TextColors)>> {
        None
    }

    fn get_header(&self, _view: ViewType, width: usize) -> String {
        format!("{1:<0$}", width, "ACTION")
    }
}

fn insert_predefined_actions(mut actions: Vec<Action>) -> Vec<Action> {
    actions.push(
        Action::new("context")
            .with_description("changes the current kube context")
            .with_aliases(&vec!["ctx"])
            .with_response(ResponseEvent::ListKubeContexts),
    );
    actions.push(
        Action::new("delete")
            .with_description("deletes selected resources")
            .with_aliases(&vec!["del"])
            .with_response(ResponseEvent::AskDeleteResources),
    );
    actions.push(
        Action::new("quit")
            .with_description("exits the application")
            .with_aliases(&vec!["q", "exit"])
            .with_response(ResponseEvent::ExitApplication),
    );

    actions
}
