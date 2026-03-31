use b4n_config::keys::KeyCommand;
use b4n_config::themes::SelectColors;
use b4n_tui::ResponseEvent;
use b4n_tui::widgets::{ErrorHighlightMode, InputValidator, ValidatorKind};

use crate::core::{SharedAppData, SharedAppDataExt, SharedBgWorker};
use crate::ui::widgets::{PatternItem, PatternsList, Picker, PickerBehaviour};

const NAMESPACE_HISTORY_SIZE: usize = 20;

pub type NamespaceSelector = Picker<NamespaceBehaviour>;

impl NamespaceSelector {
    /// Creates new [`NamespaceSelector`] instance.
    pub fn new(app_data: SharedAppData, worker: Option<SharedBgWorker>, width: u16) -> Self {
        Picker::new_picker(app_data, worker, width, NamespaceBehaviour::default())
    }

    /// Updates the list of discovered namespaces.
    pub fn set_discovered(&mut self, namespaces: Vec<String>) {
        self.behaviour_mut().set_discovered(namespaces);
    }
}

pub struct NamespaceBehaviour {
    discovered: Vec<String>,
    validator: InputValidator,
}

impl Default for NamespaceBehaviour {
    fn default() -> Self {
        Self {
            discovered: Vec::new(),
            validator: InputValidator::new(ValidatorKind::Namespace),
        }
    }
}

impl NamespaceBehaviour {
    /// Updates the list of discovered namespaces.
    pub fn set_discovered(&mut self, namespaces: Vec<String>) {
        self.discovered = namespaces;
    }
}

impl PickerBehaviour for NamespaceBehaviour {
    fn prompt(&self) -> &str {
        "namespace "
    }

    fn colors(&self, app_data: &SharedAppData) -> SelectColors {
        app_data.borrow().theme.colors.command_palette.clone()
    }

    fn reset_key_command(&self) -> KeyCommand {
        KeyCommand::CommandPaletteReset
    }

    fn cancel_response(&self) -> ResponseEvent {
        ResponseEvent::Cancelled
    }

    fn load_items(&self, app_data: &SharedAppData) -> PatternsList {
        let key_name = app_data.get_key_name(KeyCommand::NavigateComplete).to_ascii_uppercase();
        let context = &app_data.borrow().current.context;
        let mut items = PatternsList::from(app_data.borrow().history.namespace_history(context), Some(&key_name));
        for item in items.list.full_iter_mut() {
            item.data.icon = Some("");
        }

        for ns in &self.discovered {
            items.add_or_update(PatternItem::fixed(ns.clone()));
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

    fn can_remove(&self, item: Option<&PatternItem>) -> bool {
        item.is_some_and(|i| !i.is_fixed)
    }

    fn error_mode(&self) -> ErrorHighlightMode {
        ErrorHighlightMode::Value
    }

    fn validate(&mut self, value: &str) -> Option<usize> {
        self.validator.validate(value).err()
    }

    fn restores_on_cancel(&self) -> bool {
        true
    }

    fn blocks_on_error(&self) -> bool {
        true
    }

    fn has_header(&self) -> bool {
        false
    }

    fn get_response(&self, pattern: &str, highlighted: Option<&str>) -> ResponseEvent {
        if pattern.is_empty()
            && let Some(highlighted) = highlighted
        {
            ResponseEvent::ChangeNamespace(highlighted.to_owned())
        } else if !pattern.is_empty() {
            ResponseEvent::ChangeNamespace(pattern.to_owned())
        } else {
            ResponseEvent::Handled
        }
    }
}
