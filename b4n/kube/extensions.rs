use b4n_tui::ResponseEvent;
use b4n_tui::widgets::{ActionItem, ActionsListBuilder};
use kube::config::NamedContext;

use crate::kube::kinds::KindItem;

/// Extensions trait for [`ActionsListBuilder`].
pub trait ActionsListBuilderExt {
    /// Creates new [`ActionsListBuilder`] instance from [`NamedContext`] collection.
    fn from_kube_contexts(items: &[NamedContext]) -> ActionsListBuilder;

    /// Creates new [`ActionsListBuilder`] instance from [`KindItem`] collection.
    fn from_kinds(items: Option<&[KindItem]>) -> Self;
}

impl ActionsListBuilderExt for ActionsListBuilder {
    fn from_kube_contexts(items: &[NamedContext]) -> ActionsListBuilder {
        let actions = items.iter().map(|item| {
            let cluster = item.context.as_ref().map(|c| c.cluster.as_str()).unwrap_or_default();
            let uid = format!("_{}:{}_", item.name, cluster);
            let namespace = item.context.as_ref().and_then(|c| c.namespace.clone());
            ActionItem::raw(uid, "context".to_owned(), item.name.clone(), None)
                .with_description(cluster)
                .with_response(ResponseEvent::ChangeContext(item.name.clone(), namespace))
        });

        ActionsListBuilder::new(actions.collect())
    }

    fn from_kinds(items: Option<&[KindItem]>) -> Self {
        let actions = items.unwrap_or(&[]).iter().map(Into::into).collect();
        ActionsListBuilder::new(actions)
    }
}
