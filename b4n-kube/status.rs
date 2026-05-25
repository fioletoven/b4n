use k8s_openapi::serde_json::Value;
use kube::api::DynamicObject;

const STATUS_UNKNOWN: &str = "Unknown";
const STATUS_READY: &str = "Ready";
const STATUS_NOT_READY: &str = "NotReady";

/// Extracts status from a [`DynamicObject`] as a string.
pub fn from_object(object: &DynamicObject) -> &str {
    let Some(status) = object.data.get("status") else {
        return STATUS_UNKNOWN;
    };

    if let Some(phase) = status["phase"].as_str() {
        return phase;
    }

    if let Some(status) = from_conditions(status["conditions"].as_array()) {
        return status;
    }

    if let Some(desired) = status["replicas"].as_i64() {
        let ready = status["readyReplicas"].as_i64().unwrap_or(0);
        return if desired == ready { STATUS_READY } else { STATUS_NOT_READY };
    }

    STATUS_UNKNOWN
}

/// Extracts status from object's conditions.
pub fn from_conditions(conditions: Option<&Vec<Value>>) -> Option<&str> {
    if let Some(conditions) = conditions {
        for condition in conditions {
            if condition["type"].as_str() == Some(STATUS_READY) {
                let status = condition["status"].as_str().unwrap_or(STATUS_UNKNOWN);
                return if status == "True" {
                    Some(STATUS_READY)
                } else {
                    Some(condition["reason"].as_str().unwrap_or(STATUS_NOT_READY))
                };
            }
        }
    }

    None
}
