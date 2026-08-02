use b4n_kube::{ContainerType, ResourceTag};
use std::path::PathBuf;

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

/// Tries to guess selected path for the file picker basing on the `is_download` flag.
pub fn get_path_for_file_picker(path: &str, is_download: bool) -> PathBuf {
    fn default_path() -> PathBuf {
        std::env::current_dir().unwrap_or(PathBuf::from("."))
    }

    if path.trim().is_empty() {
        default_path()
    } else {
        let path = PathBuf::from(path);
        if is_download {
            path
        } else if let Some(path) = path.parent()
            && !path.as_os_str().is_empty()
        {
            PathBuf::from(path)
        } else {
            default_path()
        }
    }
}
