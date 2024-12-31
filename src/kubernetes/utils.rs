use k8s_openapi::{apimachinery::pkg::apis::meta::v1::Time, chrono::Utc};
use kube::{api::ApiResource, discovery::ApiCapabilities, Discovery};

/// Resolves name to the kubernetes resource
pub fn get_resource(discovery: &Discovery, name: &str) -> Option<(ApiResource, ApiCapabilities)> {
    discovery
        .groups()
        .flat_map(|group| group.resources_by_stability().into_iter().map(move |res| (group, res)))
        .filter(|(_, (res, _))| name.eq_ignore_ascii_case(&res.kind) || name.eq_ignore_ascii_case(&res.plural))
        .min_by_key(|(group, _res)| group.name())
        .map(|(_, res)| res)
}

/// Formats kubernetes timestamp to a human-readable string
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
