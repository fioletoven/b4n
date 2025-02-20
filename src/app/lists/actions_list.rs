use delegate::delegate;
use kube::config::NamedContext;
use std::collections::HashMap;

use crate::{
    kubernetes::resources::Kind,
    ui::{ResponseEvent, Responsive, Table, ViewType, colors::TextColors, theme::Theme, widgets::Action},
};

use super::ScrollableList;

/// UI actions list.
#[derive(Default)]
pub struct ActionsList {
    pub list: ScrollableList<Action>,
}

impl Responsive for ActionsList {
    fn process_key(&mut self, key: crossterm::event::KeyEvent) -> ResponseEvent {
        self.list.process_key(key)
    }
}

impl Table for ActionsList {
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
    /// As actions are used only in selector, we don't care to implement this.
    fn get_paged_items(&self, _theme: &Theme, _view: ViewType, _width: usize) -> Option<Vec<(String, TextColors)>> {
        None
    }

    fn get_header(&self, _view: ViewType, width: usize) -> String {
        format!("{1:<0$}", width, "ACTION")
    }
}

/// Helper to build [`ActionsList`].
#[derive(Default)]
pub struct ActionsListBuilder {
    actions: Vec<Action>,
}

impl ActionsListBuilder {
    /// Creates new [`ActionsListBuilder`] instance from the provided kinds.
    pub fn from_kinds(kinds: &ScrollableList<Kind>) -> Self {
        ActionsListBuilder {
            actions: if let Some(items) = &kinds.items {
                items.full_iter().map(|i| Action::from_kind(&i.data)).collect::<Vec<Action>>()
            } else {
                Vec::new()
            },
        }
    }

    /// Creates new [`ActionsListBuilder`] instance from the list of [`NamedContext`]s.
    pub fn from_contexts(contexts: &[NamedContext]) -> Self {
        ActionsListBuilder {
            actions: contexts.iter().map(Action::from_context).collect::<Vec<Action>>(),
        }
    }

    /// Builds the [`ActionsList`] instance.
    pub fn build(self) -> ActionsList {
        let mut list = ScrollableList::from(self.actions);
        list.sort(1, false);

        ActionsList { list }
    }

    /// Adds custom action.
    pub fn with_action(mut self, action: Action) -> Self {
        self.actions.push(action);
        self
    }

    /// Adds actions relevant to resources view.
    pub fn with_resources_actions(self, is_disconnected: bool) -> Self {
        let builder = self.with_context().with_quit();
        if !is_disconnected { builder.with_delete() } else { builder }
    }

    /// Adds `quit` action.
    pub fn with_quit(mut self) -> Self {
        self.actions.push(
            Action::new("quit")
                .with_description("exits the application")
                .with_aliases(&["q", "exit"])
                .with_response(ResponseEvent::ExitApplication),
        );
        self
    }

    /// Adds `close` action.
    pub fn with_close(mut self) -> Self {
        self.actions.push(
            Action::new("close")
                .with_description("closes the current view")
                .with_aliases(&["cancel"])
                .with_response(ResponseEvent::Cancelled),
        );
        self
    }

    /// Adds `context` action.
    pub fn with_context(mut self) -> Self {
        self.actions.push(
            Action::new("context")
                .with_description("changes the current kube context")
                .with_aliases(&["ctx"])
                .with_response(ResponseEvent::ListKubeContexts),
        );
        self
    }

    /// Adds `delete` action.
    pub fn with_delete(mut self) -> Self {
        self.actions.push(
            Action::new("delete")
                .with_description("deletes selected resources")
                .with_aliases(&["del"])
                .with_response(ResponseEvent::AskDeleteResources),
        );
        self
    }
}
