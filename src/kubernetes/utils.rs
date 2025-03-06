use k8s_openapi::{apimachinery::pkg::apis::meta::v1::Time, chrono::Utc};
use kube::{
    ResourceExt,
    api::{ApiResource, DynamicObject},
    discovery::ApiCapabilities,
};

/// Serializes kubernetes resource to YAML.
pub fn serialize_resource(resource: &mut DynamicObject) -> Result<String, serde_yaml::Error> {
    resource.managed_fields_mut().clear();
    let mut yaml = serde_yaml::to_string(resource)?;

    if let Some(index) = yaml.find("\n  managedFields: []\n") {
        yaml.replace_range(index + 1..index + 21, "");
    }

    Ok(yaml)
}

/// Formats kubernetes timestamp to a human-readable string.
pub fn format_timestamp(time: &Time) -> String {
    let duration = Utc::now().signed_duration_since(time.0);
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

/// Gets first matching [`ApiResource`] and [`ApiCapabilities`] for the resource name.  
/// Name value can be in the format `name.group`.
pub fn get_resource(list: Option<&Vec<(ApiResource, ApiCapabilities)>>, name: &str) -> Option<(ApiResource, ApiCapabilities)> {
    if name.contains('.') {
        let mut split = name.splitn(2, '.');
        get_resource_with_group(list, split.next().unwrap(), split.next().unwrap())
    } else {
        get_resource_no_group(list, name)
    }
}

/// Gets first matching [`ApiResource`] and [`ApiCapabilities`] for the resource name and group.
fn get_resource_with_group(
    list: Option<&Vec<(ApiResource, ApiCapabilities)>>,
    name: &str,
    group: &str,
) -> Option<(ApiResource, ApiCapabilities)> {
    if group.is_empty() {
        get_resource_no_group(list, name)
    } else {
        list.and_then(|discovery| {
            discovery
                .iter()
                .find(|(ar, _)| {
                    group.eq_ignore_ascii_case(&ar.group)
                        && (name.eq_ignore_ascii_case(&ar.kind) || name.eq_ignore_ascii_case(&ar.plural))
                })
                .map(|(ar, cap)| (ar.clone(), cap.clone()))
        })
    }
}

/// Gets first matching [`ApiResource`] and [`ApiCapabilities`] for the resource name ignoring group.
fn get_resource_no_group(
    list: Option<&Vec<(ApiResource, ApiCapabilities)>>,
    name: &str,
) -> Option<(ApiResource, ApiCapabilities)> {
    list.and_then(|discovery| {
        discovery
            .iter()
            .filter(|(ar, _)| name.eq_ignore_ascii_case(&ar.kind) || name.eq_ignore_ascii_case(&ar.plural))
            .min_by_key(|(ar, _)| &ar.group)
            .map(|(ar, cap)| (ar.clone(), cap.clone()))
    })
}
