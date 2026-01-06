use b4n_config::keys::{KeyBindings, KeyCommand};
use b4n_kube::{Port, PortProtocol};
use b4n_list::{BasicFilterContext, Row, ScrollableList};
use delegate::delegate;
use std::{collections::HashMap, path::PathBuf};

use crate::table::{Table, ViewType};
use crate::widgets::ActionItem;
use crate::{ResponseEvent, Responsive, TuiEvent};

/// UI actions list.
#[derive(Default)]
pub struct ActionsList {
    pub list: ScrollableList<ActionItem, BasicFilterContext>,
    header: String,
    width: usize,
}

impl Responsive for ActionsList {
    fn process_event(&mut self, event: &TuiEvent) -> ResponseEvent {
        self.list.process_event(event)
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

    fn get_column_at_position(&self, position: usize) -> Option<usize> {
        if position < self.width { Some(0) } else { None }
    }

    /// Not implemented for [`ActionsList`].
    fn toggle_sort(&mut self, _column_no: usize) {
        // pass
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
    commands: Vec<Option<KeyCommand>>,
}

impl ActionsListBuilder {
    /// Creates a new [`ActionsListBuilder`] instance.
    pub fn new(actions: Vec<ActionItem>) -> Self {
        let commands = vec![None; actions.len()];
        Self { actions, commands }
    }

    /// Creates new [`ActionsListBuilder`] instance from the list of string slices.\
    /// **Note** that items order is preserved.
    pub fn from_strings(items: &[&str]) -> Self {
        let actions = items
            .iter()
            .enumerate()
            .map(|(idx, item)| ActionItem::raw(idx.to_string(), "items".to_owned(), item.to_string(), None).with_id(idx))
            .collect();
        let commands = vec![None; items.len()];
        Self { actions, commands }
    }

    /// Creates new [`ActionsListBuilder`] instance from the list of [`PathBuf`]s.
    pub fn from_paths(themes: Vec<PathBuf>) -> Self {
        let commands = vec![None; themes.len()];
        Self {
            actions: themes.into_iter().map(ActionItem::from_path).collect(),
            commands,
        }
    }

    /// Creates new [`ActionsListBuilder`] instance from the list of [`Port`]s.
    pub fn from_resource_ports(ports: &[Port]) -> Self {
        Self {
            actions: ports
                .iter()
                .filter(|p| p.protocol == PortProtocol::TCP)
                .map(ActionItem::from_port)
                .collect(),
            commands: vec![None; ports.len()],
        }
    }

    /// Builds the [`ActionsList`] instance.\
    /// **Note** that if `key_bindings` is provided all items in the list will have an additional key hint.
    pub fn build(mut self, key_bindings: Option<&KeyBindings>) -> ActionsList {
        if let Some(key_bindings) = key_bindings {
            self.update_key_bindings(key_bindings);
        }

        let has_ids = self.actions.iter().any(|a| a.id.is_some());
        let mut list = ScrollableList::from(self.actions);

        if has_ids && let Some(items) = &mut list.items {
            items.full_sort_by(|a, b| a.data.id.cmp(&b.data.id));
        } else {
            list.sort(1, false);
        }

        ActionsList {
            list,
            ..Default::default()
        }
    }

    /// Adds aliases to the existing actions.
    pub fn with_aliases(mut self, aliases: &HashMap<String, String>) -> Self {
        for action in &mut self.actions {
            if let Some(aliases) = aliases.get(action.name()) {
                action.add_aliases(aliases.split(',').map(String::from).collect());
            }
        }

        self
    }

    /// Adds custom action.
    pub fn with_action(mut self, action: ActionItem) -> Self {
        self.actions.push(action);
        self.commands.push(None);
        self
    }

    /// Adds custom action with response [`ResponseEvent::Action`].
    pub fn with_command(mut self, command: &str, description: &str, aliases: &[&str], action: &'static str) -> Self {
        self.actions.push(
            ActionItem::new(command)
                .with_description(description)
                .with_aliases(aliases)
                .with_response(ResponseEvent::Action(action)),
        );
        self.commands.push(None);
        self
    }

    /// Adds actions relevant to resources view.
    pub fn with_resources_actions(self, is_deletable: bool) -> Self {
        let builder = self.with_context().with_theme().with_quit();
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
        self.commands.push(Some(KeyCommand::ApplicationExit));
        self
    }

    /// Adds `back` action that closes the current view.
    pub fn with_back(mut self) -> Self {
        self.actions.push(
            ActionItem::new("back")
                .with_description("closes the current view")
                .with_aliases(&["cancel", "close"])
                .with_response(ResponseEvent::Cancelled),
        );
        self.commands.push(Some(KeyCommand::NavigateBack));
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
        self.commands.push(None);
        self
    }

    /// Adds `theme` action.
    pub fn with_theme(mut self) -> Self {
        self.actions.push(
            ActionItem::new("theme")
                .with_description("selects the theme used by the application")
                .with_response(ResponseEvent::ListThemes),
        );
        self.commands.push(None);
        self
    }

    /// Adds `delete` action.
    pub fn with_delete(mut self) -> Self {
        self.actions.push(
            ActionItem::new("delete")
                .with_description("deletes selected resources")
                .with_aliases(&["del", "remove"])
                .with_response(ResponseEvent::AskDeleteResources),
        );
        self.commands.push(Some(KeyCommand::NavigateDelete));
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
        self.commands.push(Some(KeyCommand::PortForwardsOpen));
        self
    }

    /// Adds custom action.
    pub fn add_action(&mut self, action: ActionItem) {
        self.actions.push(action);
        self.commands.push(None);
    }

    fn update_key_bindings(&mut self, key_bindings: &KeyBindings) {
        let commands = key_bindings.inverted();
        for (action, command) in self.actions.iter_mut().zip(self.commands.iter()) {
            if let Some(command) = command
                && let Some(keys) = commands.get(command)
            {
                let mut keys = keys.iter().map(|k| k.to_string()).collect::<Vec<_>>();
                keys.sort();

                if !keys.is_empty() {
                    action.set_key(Some(keys.swap_remove(0)));
                }
            }
        }
    }
}
