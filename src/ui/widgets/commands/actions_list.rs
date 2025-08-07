use delegate::delegate;
use kube::config::NamedContext;
use std::{collections::HashMap, path::PathBuf};

use crate::{
    kubernetes::{
        kinds::KindItem,
        resources::{Port, PortProtocol},
    },
    ui::{
        ResponseEvent, Responsive, Table, ViewType,
        colors::TextColors,
        lists::{BasicFilterContext, ScrollableList},
        theme::Theme,
        widgets::ActionItem,
    },
};

/// UI actions list.
#[derive(Default)]
pub struct ActionsList {
    pub list: ScrollableList<ActionItem, BasicFilterContext>,
    header: String,
    width: usize,
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
            fn get_highlighted_item_uid(&self) -> Option<&str>;
            fn highlight_item_by_name(&mut self, name: &str) -> bool;
            fn highlight_item_by_name_start(&mut self, text: &str) -> bool;
            fn highlight_item_by_uid(&mut self, uid: &str) -> bool;
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

    /// Returns items from the current page in a form of text lines to display and colors for that lines.\
    /// **Note** that this is not implemented for [`ActionsList`].
    fn get_paged_items(&self, _theme: &Theme, _view: ViewType, _width: usize) -> Option<Vec<(String, TextColors)>> {
        None
    }

    fn get_header(&mut self, _view: ViewType, width: usize) -> &str {
        if self.width == width {
            return &self.header;
        }

        self.header = format!("{1:<0$}", width, "ACTION");
        self.width = width;

        &self.header
    }
}

/// Helper to build [`ActionsList`].
#[derive(Default)]
pub struct ActionsListBuilder {
    actions: Vec<ActionItem>,
}

impl ActionsListBuilder {
    /// Creates new [`ActionsListBuilder`] instance from the provided kinds.
    pub fn from(actions: Vec<ActionItem>) -> Self {
        ActionsListBuilder { actions }
    }

    /// Creates new [`ActionsListBuilder`] instance from the provided kinds.
    pub fn from_kinds(kinds: Option<&[KindItem]>) -> Self {
        ActionsListBuilder {
            actions: if let Some(items) = &kinds {
                items.iter().map(ActionItem::from_kind).collect::<Vec<ActionItem>>()
            } else {
                Vec::new()
            },
        }
    }

    /// Creates new [`ActionsListBuilder`] instance from the list of [`NamedContext`]s.
    pub fn from_contexts(contexts: &[NamedContext]) -> Self {
        ActionsListBuilder {
            actions: contexts.iter().map(ActionItem::from_context).collect::<Vec<ActionItem>>(),
        }
    }

    /// Creates new [`ActionsListBuilder`] instance from the list of [`PathBuf`]s.
    pub fn from_paths(themes: Vec<PathBuf>) -> Self {
        ActionsListBuilder {
            actions: themes.into_iter().map(ActionItem::from_path).collect::<Vec<ActionItem>>(),
        }
    }

    /// Creates new [`ActionsListBuilder`] instance from the list of [`Port`]s.
    pub fn from_resource_ports(ports: &[Port]) -> Self {
        ActionsListBuilder {
            actions: ports
                .iter()
                .filter(|p| p.protocol == PortProtocol::TCP)
                .map(ActionItem::from_port)
                .collect::<Vec<ActionItem>>(),
        }
    }

    /// Builds the [`ActionsList`] instance.
    pub fn build(self) -> ActionsList {
        let mut list = ScrollableList::from(self.actions);
        list.sort(1, false);

        ActionsList {
            list,
            ..Default::default()
        }
    }

    /// Returns vector of [`ActionItem`]s that can be used later to build [`ActionsList`].
    pub fn to_vec(self) -> Vec<ActionItem> {
        self.actions
    }

    /// Adds custom action.
    pub fn with_action(mut self, action: ActionItem) -> Self {
        self.actions.push(action);
        self
    }

    /// Adds actions relevant to resources view.
    pub fn with_resources_actions(self, is_deletable: bool) -> Self {
        let builder = self.with_context().with_forwards().with_theme().with_quit();
        if is_deletable { builder.with_delete() } else { builder }
    }

    /// Adds `quit` action.
    pub fn with_quit(mut self) -> Self {
        self.actions.push(
            ActionItem::new("quit")
                .with_description("exits the application")
                .with_aliases(&["q", "exit"])
                .with_response(ResponseEvent::ExitApplication),
        );
        self
    }

    /// Adds `close` action.
    pub fn with_close(mut self) -> Self {
        self.actions.push(
            ActionItem::new("close")
                .with_description("closes the current view")
                .with_aliases(&["cancel"])
                .with_response(ResponseEvent::Cancelled),
        );
        self
    }

    /// Adds `context` action.
    pub fn with_context(mut self) -> Self {
        self.actions.push(
            ActionItem::new("context")
                .with_description("changes the current kube context")
                .with_aliases(&["ctx"])
                .with_response(ResponseEvent::ListKubeContexts),
        );
        self
    }

    /// Adds `theme` action.
    pub fn with_theme(mut self) -> Self {
        self.actions.push(
            ActionItem::new("theme")
                .with_description("selects the theme used by the application")
                .with_response(ResponseEvent::ListThemes),
        );
        self
    }

    /// Adds `delete` action.
    pub fn with_delete(mut self) -> Self {
        self.actions.push(
            ActionItem::new("delete")
                .with_description("deletes selected resources")
                .with_aliases(&["del"])
                .with_response(ResponseEvent::AskDeleteResources),
        );
        self
    }

    /// Adds `show port forwards` action.
    pub fn with_forwards(mut self) -> Self {
        self.actions.push(
            ActionItem::new("show port forwards")
                .with_description("shows active port forwards")
                .with_aliases(&["port", "pf", "forward"])
                .with_response(ResponseEvent::ShowPortForwards),
        );
        self
    }
}
