use k8s_openapi::{
    apimachinery::pkg::apis::meta::v1::Time,
    chrono::{DateTime, Utc},
};
use kube::{
    ResourceExt,
    api::{ApiResource, DynamicObject},
    discovery::ApiCapabilities,
};

use crate::core::DiscoveryList;

use super::Kind;

/// Serializes kubernetes resource to YAML.
pub fn serialize_resource(resource: &mut DynamicObject) -> Result<String, serde_yaml::Error> {
    resource.managed_fields_mut().clear();
    let mut yaml = serde_yaml::to_string(resource)?;

    if let Some(index) = yaml.find("\n  managedFields: []\n") {
        yaml.replace_range(index + 1..index + 21, "");
    }

    Ok(yaml)
}

/// Gets [`DynamicObject`]'s UID.
pub fn get_object_uid(object: &DynamicObject) -> String {
    object.uid().clone().unwrap_or_else(|| {
        format!(
            "_{}{}_",
            object.name_any(),
            object.metadata.namespace.as_deref().unwrap_or_default()
        )
    })
}

/// Formats kubernetes timestamp to a human-readable string.
#[inline]
pub fn format_timestamp(time: &Time) -> String {
    format_datetime(&time.0)
}

/// Formats datetime to a human-readable string.
pub fn format_datetime(time: &DateTime<Utc>) -> String {
    let duration = Utc::now().signed_duration_since(time);
    let days = duration.num_days();
    let hours = duration.num_hours() - (days * 24);

    if days > 0 {
        format!("{days}d{hours:0>2}h")
    } else {
        let minutes = duration.num_minutes() - (days * 1_440) - (hours * 60);
        if hours > 0 {
            format!("{hours}h{minutes:0>2}m")
        } else {
            let secs = duration.num_seconds() - (days * 86_400) - (hours * 3_600) - (minutes * 60);
            if minutes > 0 {
                format!("{minutes}m{secs:0>2}s")
            } else {
                format!("{secs}s")
            }
        }
    }
}

/// Gets first matching plural resource name for the specified `kind`.
pub fn get_plural<'a>(list: Option<&'a DiscoveryList>, kind: &Kind) -> Option<&'a str> {
    if let Some(resource) = get_resource_internal(list, kind) {
        Some(&resource.0.plural)
    } else {
        None
    }
}

/// Gets first matching [`ApiResource`] and [`ApiCapabilities`] for the specified `kind`.
pub fn get_resource(list: Option<&DiscoveryList>, kind: &Kind) -> Option<(ApiResource, ApiCapabilities)> {
    get_resource_internal(list, kind).cloned()
}

pub fn get_resource_internal<'a>(list: Option<&'a DiscoveryList>, kind: &Kind) -> Option<&'a (ApiResource, ApiCapabilities)> {
    if kind.has_version() {
        get_resource_with_version(list, kind.name(), kind.api_version())
    } else if kind.has_group() && !kind.group().is_empty() {
        get_resource_with_group(list, kind.name(), kind.group())
    } else {
        get_resource_no_group(list, kind.as_str())
    }
}

/// Gets first matching [`ApiResource`] and [`ApiCapabilities`] for the resource `kind` and `api_version`.
fn get_resource_with_version<'a>(
    list: Option<&'a DiscoveryList>,
    kind: &str,
    api_version: &str,
) -> Option<&'a (ApiResource, ApiCapabilities)> {
    list?.iter().find(|(ar, _)| {
        api_version.eq_ignore_ascii_case(&ar.api_version)
            && (kind.eq_ignore_ascii_case(&ar.kind) || kind.eq_ignore_ascii_case(&ar.plural))
    })
}

/// Gets first matching [`ApiResource`] and [`ApiCapabilities`] for the resource `kind` and `group`.
fn get_resource_with_group<'a>(
    list: Option<&'a DiscoveryList>,
    kind: &str,
    group: &str,
) -> Option<&'a (ApiResource, ApiCapabilities)> {
    list?.iter().find(|(ar, _)| {
        group.eq_ignore_ascii_case(&ar.group) && (kind.eq_ignore_ascii_case(&ar.kind) || kind.eq_ignore_ascii_case(&ar.plural))
    })
}

/// Gets first matching [`ApiResource`] and [`ApiCapabilities`] for the resource `kind` ignoring `group`.
fn get_resource_no_group<'a>(list: Option<&'a DiscoveryList>, kind: &str) -> Option<&'a (ApiResource, ApiCapabilities)> {
    list?
        .iter()
        .filter(|(ar, _)| kind.eq_ignore_ascii_case(&ar.kind) || kind.eq_ignore_ascii_case(&ar.plural))
        .min_by_key(|(ar, _)| &ar.group)
}
