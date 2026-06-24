use b4n_config::Plugins;
use b4n_kube::{Kind, Namespace, ResourceRef};

use crate::{ResponseEvent, widgets::ActionItem};

/// Execution context for a plugin.
#[derive(Default, Debug, Clone, PartialEq)]
pub struct PluginContext {
    pub context: String,
    pub kind: Kind,
    pub namespace: Namespace,
    pub highlighted: Option<ResourceRef>,
    pub selected: Vec<ResourceRef>,
    pub columns: Vec<String>,
    pub values: Vec<Vec<String>>,
}

/// Plugins extension trait.
pub trait PluginsExt {
    /// Creates vec of [`ActionItem`]s.
    fn to_actions(&self, scope: &str, is_highlighted: bool, is_selected: bool) -> Vec<ActionItem>;
}

impl PluginsExt for Plugins {
    fn to_actions(&self, scope: &str, is_highlighted: bool, is_selected: bool) -> Vec<ActionItem> {
        let mut actions = Vec::new();
        let plugins = self.iter().filter(|p| {
            (!p.highlighted || p.highlighted == is_highlighted)
                && (!p.selected || p.selected == is_selected)
                && p.scopes.iter().any(|s| s == scope)
        });

        for plugin in plugins {
            let mut action = ActionItem::new(&plugin.name)
                .with_description(&plugin.description)
                .with_aliases(&plugin.aliases)
                .with_response(ResponseEvent::PluginAction(
                    plugin.id.clone(),
                    plugin.highlighted,
                    plugin.selected,
                ))
                .with_icon(Some(""));

            if !plugin.shortcut.is_default() {
                action.set_key(Some(plugin.shortcut.to_string()));
            }

            actions.push(action);
        }

        actions
    }
}
