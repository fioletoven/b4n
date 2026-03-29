use b4n_config::keys::KeyCommand;
use b4n_config::themes::SelectColors;
use b4n_tui::ResponseEvent;

use crate::core::{SharedAppData, SharedAppDataExt, SharedBgWorker};
use crate::ui::widgets::{PatternItem, PatternsList, Picker, PickerBehaviour};

const NAMESPACE_HISTORY_SIZE: usize = 20;

pub type NamespaceSelector = Picker<NamespaceBehaviour>;

impl NamespaceSelector {
    pub fn new(app_data: SharedAppData, worker: Option<SharedBgWorker>, width: u16) -> Self {
        Picker::new_picker(app_data, worker, width, NamespaceBehaviour::new())
    }

    /// Update the list of discovered namespaces.
    pub fn set_discovered(&mut self, namespaces: Vec<String>) {
        self.behaviour_mut().set_discovered(namespaces);
    }
}

pub struct NamespaceBehaviour {
    discovered: Vec<String>,
}

impl NamespaceBehaviour {
    pub fn new() -> Self {
        Self { discovered: Vec::new() }
    }

    pub fn set_discovered(&mut self, namespaces: Vec<String>) {
        self.discovered = namespaces;
    }

    pub fn discovered(&self) -> &[String] {
        &self.discovered
    }
}

impl PickerBehaviour for NamespaceBehaviour {
    fn prompt(&self) -> &str {
        " "
    }

    fn colors(&self) -> &SelectColors {
        todo!()
    }

    fn reset_key_command(&self) -> KeyCommand {
        KeyCommand::FilterReset
    }

    fn cancel_response(&self) -> ResponseEvent {
        ResponseEvent::Cancelled
    }

    fn load_items(&self, app_data: &SharedAppData) -> PatternsList {
        let key_name = app_data.get_key_name(KeyCommand::NavigateComplete).to_ascii_uppercase();
        let context = &app_data.borrow().current.context;
        let mut items = PatternsList::from(app_data.borrow().history.namespace_history(context), Some(&key_name));

        for ns in &self.discovered {
            items.add_or_update(PatternItem::fixed(ns.clone(), Some("discovered".to_string())));
        }

        items
    }

    fn add_item(&self, app_data: &SharedAppData, item: &str) {
        let context = app_data.borrow().current.context.clone();
        app_data
            .borrow_mut()
            .history
            .put_namespace_history_item(&context, item.into(), NAMESPACE_HISTORY_SIZE);
    }

    fn remove_item(&self, app_data: &SharedAppData, item: &str) -> bool {
        let context = app_data.borrow().current.context.clone();
        app_data
            .borrow_mut()
            .history
            .remove_namespace_history_item(&context, item)
            .is_some()
    }

    fn restores_on_cancel(&self) -> bool {
        true
    }
}
