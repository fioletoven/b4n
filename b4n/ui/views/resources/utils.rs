use b4n_kube::{ContainerType, ResourceTag};
use b4n_tui::widgets::{ActionItem, ActionsList, ActionsListBuilder};

/// Gets names for all containers in the list of resource tags.
pub fn get_all_containers_from_resource_tags(tags: &[ResourceTag]) -> Vec<String> {
    tags.iter()
        .filter_map(|t| match t {
            ResourceTag::Container(name, _, _) => Some(name.to_ascii_lowercase()),
            _ => None,
        })
        .collect()
}

/// Gets names for all non-ephemeral containers in the list of resource tags.
pub fn get_target_containers_from_resource_tags(tags: &[ResourceTag]) -> Vec<String> {
    tags.iter()
        .filter_map(|t| match t {
            ResourceTag::Container(name, kind, _) if *kind != ContainerType::Ephemeral => Some(name.to_ascii_lowercase()),
            _ => None,
        })
        .collect()
}

/// Converts non-ephemeral container resource tags into actions list.
pub fn get_actions_from_resource_tags(tags: Vec<ResourceTag>) -> ActionsList {
    let actions = tags.into_iter().filter_map(|t| match t {
        ResourceTag::Container(name, kind, _) if kind != ContainerType::Ephemeral => Some(
            ActionItem::raw(format!("_{name}:{kind}_"), "container".to_owned(), name, None)
                .with_description(&kind.to_string().to_ascii_lowercase()),
        ),
        _ => None,
    });

    ActionsListBuilder::new(actions.collect()).build(None)
}
