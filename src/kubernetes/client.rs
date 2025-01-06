use kube::{
    api::{ApiResource, DynamicObject},
    config::Kubeconfig,
    discovery::{ApiCapabilities, Scope},
    Api, Client, Config,
};
use std::ops::{Deref, DerefMut};
use thiserror;
use tracing::error;

/// Possible errors from building kubernetes client.
#[derive(thiserror::Error, Debug)]
pub enum ClientError {
    /// Failed to determine users home directory.
    #[error("failed to determine users home directory")]
    HomeDirNotFound,

    /// Failed to process kube configuration.
    #[error("failed to process kube configuration")]
    KubeconfigError(#[from] kube::config::KubeconfigError),

    /// Failed to build kubernetes client.
    #[error("failed to build kubernetes client")]
    KubeError(#[from] kube::Error),
}

/// Wrapper for the kubernetes [`Client`].
pub struct KubernetesClient {
    /// Kubernetes client.
    client: Client,

    /// Context used by the kubernetes client.
    context: String,

    /// Kubernetes API version that the client is connected to.
    k8s_version: String,
}

impl KubernetesClient {
    /// Creates new [`KubernetesClient`] instance.
    pub async fn new(kube_context: Option<&str>, fallback_to_default: bool) -> Result<Self, ClientError> {
        let (client, context) = get_client_fallback(kube_context, fallback_to_default).await?;
        let k8s_version = client.apiserver_version().await?.git_version.to_owned();

        Ok(Self {
            client,
            context,
            k8s_version,
        })
    }

    /// Changes kube context for [`KubernetesClient`] which results in creating new kubernetes client.
    pub async fn change_context(&mut self, new_kube_context: Option<&str>) -> Result<(), ClientError> {
        let (client, context) = get_client(new_kube_context).await?;

        self.k8s_version = client.apiserver_version().await?.git_version.to_owned();
        self.context = context;
        self.client = client;

        Ok(())
    }

    /// Returns cloned kubernetes client that can be consumed.
    pub fn get_client(&self) -> Client {
        self.client.clone()
    }

    /// Returns [`Api`] for the currently held kubernetes client.
    pub fn get_api(&self, ar: ApiResource, caps: ApiCapabilities, ns: Option<&str>, all: bool) -> Api<DynamicObject> {
        get_dynamic_api(ar, caps, self.client.clone(), ns, all)
    }

    /// Returns kube context name for the currently held kubernetes client.
    pub fn context(&self) -> &str {
        &self.context
    }

    /// Returns kubernetes API version.
    pub fn k8s_version(&self) -> &str {
        &self.k8s_version
    }
}

impl Deref for KubernetesClient {
    type Target = Client;

    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

impl DerefMut for KubernetesClient {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.client
    }
}

/// Gets dynamic api client for given `resource` and `namespace`.
pub fn get_dynamic_api(
    ar: ApiResource,
    caps: ApiCapabilities,
    client: Client,
    ns: Option<&str>,
    all: bool,
) -> Api<DynamicObject> {
    if caps.scope == Scope::Cluster || all {
        Api::all_with(client, &ar)
    } else if let Some(namespace) = ns {
        Api::namespaced_with(client, namespace, &ar)
    } else {
        Api::default_namespaced_with(client, &ar)
    }
}

/// Creates kubernetes client and returns it together with used context.  
/// If provided context is not valid it can try the default one.
async fn get_client_fallback(kube_context: Option<&str>, try_default: bool) -> Result<(Client, String), ClientError> {
    match get_client(kube_context).await {
        Ok(result) => Ok(result),
        Err(error) => {
            if try_default {
                error!("{}, fallback to the default context", error);
                get_client(None).await
            } else {
                Err(error)
            }
        }
    }
}

/// Creates kubernetes client and returns it together with used context.
async fn get_client(kube_context: Option<&str>) -> Result<(Client, String), ClientError> {
    match kube_context {
        Some(ctx) => Ok((get_client_for_context(ctx).await?, ctx.to_owned())),
        None => Ok((
            Client::try_default().await?,
            get_kube_config()?.current_context.unwrap_or_default(),
        )),
    }
}

/// Creates kubernetes client for the provided context.
async fn get_client_for_context(kube_context: &str) -> Result<Client, ClientError> {
    let kube_config = get_kube_config()?;
    let kube_config_options = kube::config::KubeConfigOptions {
        context: Some(String::from(kube_context)),
        user: None,
        cluster: None,
    };
    let config = Config::from_custom_kubeconfig(kube_config, &kube_config_options).await?;

    Ok(Client::try_from(config)?)
}

/// Returns kube config.
fn get_kube_config() -> Result<Kubeconfig, ClientError> {
    let mut kube_config_path = dirs::home_dir().ok_or(ClientError::HomeDirNotFound)?;
    kube_config_path.push(".kube/config");

    Ok(Kubeconfig::read_from(kube_config_path)?)
}
