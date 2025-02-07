use kube::{
    api::{ApiResource, DynamicObject},
    config::{Kubeconfig, NamedContext},
    discovery::{ApiCapabilities, Scope},
    Api, Client, Config,
};
use std::{
    ops::{Deref, DerefMut},
    path::{self, PathBuf},
};
use thiserror;
use tokio::{fs::File, io::AsyncReadExt};
use tracing::error;

/// Possible errors from building kubernetes client.
#[derive(thiserror::Error, Debug)]
pub enum ClientError {
    /// Failed to determine users home directory.
    #[error("failed to determine users home directory")]
    HomeDirNotFound,

    /// Kube config file not found.
    #[error("kube config file not found")]
    KubeConfigNotFound,

    /// Context not found in kube config.
    #[error("context not found in kube config")]
    ContextNotFound,

    /// Failed to read kube configuration.
    #[error("failed to read kube configuration")]
    IoError(#[from] std::io::Error),

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

    /// Kube config path.
    kube_config_path: Option<String>,

    /// Context used by the kubernetes client.
    context: String,

    /// Kubernetes API version that the client is connected to.
    k8s_version: String,
}

impl KubernetesClient {
    /// Creates new [`KubernetesClient`] instance.
    pub async fn new(
        kube_config_path: Option<&str>,
        kube_context: Option<&str>,
        fallback_to_default: bool,
    ) -> Result<Self, ClientError> {
        let (kube_config, kube_config_path) = get_kube_config(kube_config_path).await?;
        let (client, context) = get_client_fallback(kube_config, kube_context, fallback_to_default).await?;
        let k8s_version = client.apiserver_version().await?.git_version.to_owned();

        Ok(Self {
            client,
            kube_config_path,
            context,
            k8s_version,
        })
    }

    /// Changes kube context for [`KubernetesClient`] which results in creating new kubernetes client.
    pub async fn change_context(&mut self, new_kube_context: Option<&str>) -> Result<(), ClientError> {
        let (kube_config, _) = get_kube_config(self.kube_config_path.as_deref()).await?;
        let (client, context) = get_client(kube_config, new_kube_context).await?;

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

    /// Returns path to kube config used to create this client.
    pub fn kube_config_path(&self) -> Option<&str> {
        self.kube_config_path.as_deref()
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

/// Returns matching context from the kube config for the provided one.  
/// **Note** that it can `fallback_to_default` if the provided contex is not found in kube config.
pub async fn get_context(
    kube_config_path: Option<&str>,
    kube_context: Option<&str>,
    fallback_to_default: bool,
) -> Result<(Option<String>, Option<String>), ClientError> {
    let (kube_config, kube_config_path) = get_kube_config(kube_config_path).await?;
    let Some(context) = kube_context else {
        return Ok((kube_config.current_context, kube_config_path));
    };

    let context = kube_config.contexts.into_iter().find(|c| c.name == context);
    if let Some(context) = context {
        Ok((Some(context.name), kube_config_path))
    } else if fallback_to_default {
        Ok((kube_config.current_context, kube_config_path))
    } else {
        Ok((None, kube_config_path))
    }
}

/// Returns contexts from the kube config.
pub async fn list_contexts(kube_config_path: Option<&str>) -> Result<Vec<NamedContext>, ClientError> {
    let (kube_config, _) = get_kube_config(kube_config_path).await?;
    Ok(kube_config.contexts)
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
async fn get_client_fallback(
    kube_config: Kubeconfig,
    kube_context: Option<&str>,
    try_default: bool,
) -> Result<(Client, String), ClientError> {
    if let Some(context) = get_context_internal(&kube_config, kube_context) {
        Ok((get_client_for_context(kube_config, &context).await?, context))
    } else if try_default {
        error!("context '{:?}' not found, fallback to the default one", kube_context);
        get_client(kube_config, None).await
    } else {
        Err(ClientError::ContextNotFound)
    }
}

/// Creates kubernetes client and returns it together with used context.
async fn get_client(kube_config: Kubeconfig, kube_context: Option<&str>) -> Result<(Client, String), ClientError> {
    if let Some(context) = get_context_internal(&kube_config, kube_context) {
        Ok((get_client_for_context(kube_config, &context).await?, context))
    } else {
        Err(ClientError::ContextNotFound)
    }
}

/// Creates kubernetes client for the provided [`Kubeconfig`] and context.
async fn get_client_for_context(kube_config: Kubeconfig, kube_context: &str) -> Result<Client, ClientError> {
    let kube_config_options = kube::config::KubeConfigOptions {
        context: Some(String::from(kube_context)),
        user: None,
        cluster: None,
    };
    let config = Config::from_custom_kubeconfig(kube_config, &kube_config_options).await?;

    Ok(Client::try_from(config)?)
}

/// Returns provided context (or default one if `None` specified).
fn get_context_internal(kube_config: &Kubeconfig, kube_context: Option<&str>) -> Option<String> {
    let Some(context) = kube_context else {
        return kube_config.current_context.as_ref().map(String::from);
    };

    let context = kube_config.contexts.iter().find(|c| c.name == context);
    context.map(|context| context.name.clone())
}

/// Returns kube config.
async fn get_kube_config(kube_config_path: Option<&str>) -> Result<(Kubeconfig, Option<String>), ClientError> {
    let path = kube_config_path.map_or(
        dirs::home_dir()
            .map(|h| h.join(".kube").join("config"))
            .ok_or(ClientError::HomeDirNotFound)?,
        PathBuf::from,
    );

    if !path.exists() {
        return Err(ClientError::KubeConfigNotFound);
    }

    let path = path::absolute(path)?;
    let path_result = if kube_config_path.is_some() {
        Some(path.to_str().unwrap_or_default().to_string())
    } else {
        None
    };
    let mut file = File::open(path).await?;

    let mut kube_config_str = String::new();
    file.read_to_string(&mut kube_config_str).await?;

    Ok((Kubeconfig::from_yaml(&kube_config_str)?, path_result))
}
