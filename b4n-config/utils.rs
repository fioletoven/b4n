use serde::{Serialize, Serializer, ser::SerializeMap};
use std::collections::HashMap;

/// Helper function to sort `HashMap` before serialization.
#[allow(clippy::ref_option)]
pub fn sorted_map<K, V, S>(value: &Option<HashMap<K, V>>, serializer: S) -> Result<S::Ok, S::Error>
where
    K: Ord + Serialize,
    V: Serialize,
    S: Serializer,
{
    match value {
        None => serializer.serialize_none(),
        Some(map) => {
            let mut entries: Vec<(&K, &V)> = map.iter().collect();
            entries.sort_by_key(|&(k, _)| k);

            let mut map = serializer.serialize_map(Some(entries.len()))?;
            for (k, v) in entries {
                map.serialize_entry(k, v)?;
            }

            map.end()
        },
    }
}
