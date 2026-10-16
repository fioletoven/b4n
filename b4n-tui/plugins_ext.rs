use b4n_config::Plugins;

use crate::ResponseEvent;
use crate::widgets::{ActionItem, ActionItemKind};

/// Plugins extension trait.
pub trait PluginsExt {
    /// Creates vec of [`ActionItem`]s.
    fn to_actions(&self, scope: &str, is_highlighted: bool, is_selected: bool, is_read_only: bool) -> Vec<ActionItem>;
}

impl PluginsExt for Plugins {
    fn to_actions(&self, scope: &str, is_highlighted: bool, is_selected: bool, is_read_only: bool) -> Vec<ActionItem> {
        let mut actions = Vec::new();
        let plugins = self
            .iter()
            .filter(|p| !(is_read_only && p.confirm) && p.in_scope_for(scope, is_highlighted, is_selected));

        for plugin in plugins {
            let mut action = ActionItem::new(&plugin.name, ActionItemKind::Plugin)
                .with_description(&plugin.description)
                .with_aliases(&plugin.aliases)
                .with_response(ResponseEvent::PluginAction(plugin.into()));

            if !plugin.shortcut.is_default() {
                action.set_key(Some(plugin.shortcut.to_string()));
            }

            actions.push(action);
        }

        actions
    }
}
