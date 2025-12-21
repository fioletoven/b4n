/// Propagation policy for resources deletion.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum PropagationPolicy {
    #[default]
    None,
    Orphan,
    Background,
    Foreground,
}

impl From<&str> for PropagationPolicy {
    fn from(value: &str) -> Self {
        match value {
            "Orphan" => PropagationPolicy::Orphan,
            "Background" => PropagationPolicy::Background,
            "Foreground" => PropagationPolicy::Foreground,
            _ => PropagationPolicy::None,
        }
    }
}

impl From<PropagationPolicy> for Option<kube::api::PropagationPolicy> {
    fn from(value: PropagationPolicy) -> Self {
        match value {
            PropagationPolicy::Orphan => Some(kube::api::PropagationPolicy::Orphan),
            PropagationPolicy::Background => Some(kube::api::PropagationPolicy::Background),
            PropagationPolicy::Foreground => Some(kube::api::PropagationPolicy::Foreground),
            PropagationPolicy::None => None,
        }
    }
}
