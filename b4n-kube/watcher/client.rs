use kube::api::{ApiResource, DynamicObject};
use kube::discovery::ApiCapabilities;
use kube::{Api, Client};

use crate::{Namespace, client::get_dynamic_api};

/// Internal background observer struct to keep Kubernetes client.\
/// It allows for easy namespace change (for fallback purposes).
pub struct ResourceClient {
    client: Client,
    ar: ApiResource,
    cap: ApiCapabilities,
    ns: Namespace,
}

impl ResourceClient {
    /// Creates new [`ResourceClient`] instance.
    pub fn new(client: Client, ar: ApiResource, cap: ApiCapabilities, namespace: Namespace) -> Self {
        Self {
            client,
            ar,
            cap,
            ns: namespace,
        }
    }

    /// Sets new namespace for the client.
    pub fn set_namespace(&mut self, namespace: Namespace) {
        self.ns = namespace;
    }

    /// Returns new [`Api`] object.
    pub fn get_api(&self) -> Api<DynamicObject> {
        get_dynamic_api(
            &self.ar,
            &self.cap,
            self.client.clone(),
            self.ns.as_option(),
            self.ns.is_all(),
        )
    }
}
