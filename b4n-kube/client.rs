use http::Uri;
use kube::api::{ApiResource, DynamicObject};
use kube::config::{Kubeconfig, NamedContext};
use kube::discovery::{ApiCapabilities, Scope};
use kube::{Api, Client, Config};
use std::ops::{Deref, DerefMut};
use std::path::{self, PathBuf};
use std::str::FromStr;
use thiserror;
use tokio::{fs::File, io::AsyncReadExt};

/// Possible errors from building kubernetes client.
#[derive(thiserror::Error, Debug)]
pub enum ClientError {
    /// Failed to determine user's home directory.
    #[error("failed to determine user's home directory")]
    HomeDirNotFound,

    /// Kube config file not found.
    #[error("kube config file not found")]
    KubeConfigNotFound,

    /// Context not found in kube config.
    #[error("context not found in kube config")]
    ContextNotFound,

    /// Failed to read kube configuration.
    #[error("cannot read kube config: {0}")]
    IoError(#[from] std::io::Error),

    /// Failed to process kube configuration.
    #[error("cannot build kube config: {0}")]
    KubeconfigError(#[from] kube::config::KubeconfigError),

    /// Failed to build kubernetes client.
    #[error("cannot create client: {0}")]
    KubeError(#[from] kube::Error),
}

/// Options for the Kubernetes client.
#[derive(Clone)]
pub struct ClientOptions {
    /// Cluster to use instead of the one set in context.
    pub cluster: Option<String>,

    /// User to use instead of the one set in context
    pub user: Option<String>,

    /// Allow insecure connections (do not verify TLS certificate).
    pub allow_insecure: bool,
}

impl ClientOptions {
    pub fn new(allow_insecure: bool) -> Self {
        Self {
            cluster: None,
            user: None,
            allow_insecure,
        }
    }
}

/// Holds simplified context info.
pub struct ContextInfo {
    pub name: String,
    pub namespace: Option<String>,
}

impl From<&NamedContext> for ContextInfo {
    fn from(value: &NamedContext) -> Self {
        Self {
            name: value.name.clone(),
            namespace: value.context.as_ref().and_then(|c| c.namespace.clone()),
        }
    }
}

/// Wrapper for the kubernetes [`Client`].
pub struct KubernetesClient {
    /// Kubernetes client.
    client: Client,

    /// Kube config path.
    kubeconfig_path: Option<String>,

    /// Context used by the kubernetes client.
    context: String,

    /// Kubernetes API version that the client is connected to.
    k8s_version: String,
}

impl KubernetesClient {
    /// Creates new [`KubernetesClient`] instance.
    pub async fn new(kubeconfig_path: Option<&str>, context: Option<&str>, options: ClientOptions) -> Result<Self, ClientError> {
        let (kubeconfig, kubeconfig_path) = get_kubeconfig(kubeconfig_path).await?;
        let (client, context) = get_client(kubeconfig, context, options).await?;
        let k8s_version = client.apiserver_version().await?.git_version.clone();

        Ok(Self {
            client,
            kubeconfig_path,
            context,
            k8s_version,
        })
    }

    /// Changes kube context for [`KubernetesClient`] which results in creating new kubernetes client.\
    /// **Note** that it do not preserve `cluster` or `user` options.
    pub async fn change_context(&mut self, new_kube_context: Option<&str>, allow_insecure: bool) -> Result<(), ClientError> {
        let (kubeconfig, _) = get_kubeconfig(self.kubeconfig_path.as_deref()).await?;
        let (client, context) = get_client(kubeconfig, new_kube_context, ClientOptions::new(allow_insecure)).await?;

        self.k8s_version.clone_from(&client.apiserver_version().await?.git_version);
        self.context = context;
        self.client = client;

        Ok(())
    }

    /// Returns cloned kubernetes client that can be consumed.
    pub fn get_client(&self) -> Client {
        self.client.clone()
    }

    /// Returns [`Api`] for the currently held kubernetes client.
    pub fn get_api(&self, ar: &ApiResource, caps: &ApiCapabilities, ns: Option<&str>, all: bool) -> Api<DynamicObject> {
        get_dynamic_api(ar, caps, self.client.clone(), ns, all)
    }

    /// Returns path to kube config used to create this client.
    pub fn kubeconfig_path(&self) -> Option<&str> {
        self.kubeconfig_path.as_deref()
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

/// Resolves `kubeconfig` path and checks if it exists.
pub fn resolve_kubeconfig_path(kubeconfig_path: Option<&str>) -> Result<PathBuf, ClientError> {
    let path = kubeconfig_path.map_or(
        std::env::home_dir()
            .map(|h| h.join(".kube").join("config"))
            .ok_or(ClientError::HomeDirNotFound)?,
        PathBuf::from,
    );

    if !path.exists() {
        return Err(ClientError::KubeConfigNotFound);
    }

    Ok(path::absolute(path)?)
}

/// Returns matching context from the kube config for the provided one.\
/// **Note** that it can `fallback_to_default` if the provided context is not found in kube config.
pub async fn get_context(
    kubeconfig_path: Option<&str>,
    context: Option<&str>,
    fallback_to_default: bool,
) -> Result<(Option<ContextInfo>, Option<String>), ClientError> {
    let (kubeconfig, kubeconfig_path) = get_kubeconfig(kubeconfig_path).await?;
    if let Some(context_name) = context
        && let Some(context) = kubeconfig.contexts.iter().find(|c| c.name == context_name)
    {
        Ok((Some(context.into()), kubeconfig_path))
    } else if fallback_to_default
        && let Some(context_name) = kubeconfig.current_context.as_deref()
        && let Some(context) = kubeconfig.contexts.iter().find(|c| c.name == context_name)
    {
        Ok((Some(context.into()), kubeconfig_path))
    } else {
        Ok((None, kubeconfig_path))
    }
}

/// Returns contexts from the kube config.
pub async fn list_contexts(kubeconfig_path: Option<&str>) -> Result<Vec<NamedContext>, ClientError> {
    let (kubeconfig, _) = get_kubeconfig(kubeconfig_path).await?;
    Ok(kubeconfig.contexts)
}

/// Gets dynamic api client for given `resource` and `namespace`.
pub fn get_dynamic_api(
    ar: &ApiResource,
    caps: &ApiCapabilities,
    client: Client,
    ns: Option<&str>,
    all: bool,
) -> Api<DynamicObject> {
    if caps.scope == Scope::Cluster || all {
        Api::all_with(client, ar)
    } else if let Some(namespace) = ns {
        Api::namespaced_with(client, namespace, ar)
    } else {
        Api::default_namespaced_with(client, ar)
    }
}

/// Creates kubernetes client and returns it together with used context.
async fn get_client(
    kubeconfig: Kubeconfig,
    context: Option<&str>,
    options: ClientOptions,
) -> Result<(Client, String), ClientError> {
    if let Some(context) = get_context_internal(&kubeconfig, context) {
        Ok((get_client_for_context(kubeconfig, &context, options).await?, context))
    } else {
        Err(ClientError::ContextNotFound)
    }
}

/// Creates kubernetes client for the provided [`Kubeconfig`] and context.
async fn get_client_for_context(kubeconfig: Kubeconfig, context: &str, options: ClientOptions) -> Result<Client, ClientError> {
    let kubeconfig_options = kube::config::KubeConfigOptions {
        context: Some(String::from(context)),
        user: options.user,
        cluster: options.cluster,
    };

    let mut config = Config::from_custom_kubeconfig(kubeconfig, &kubeconfig_options).await?;
    config.accept_invalid_certs = options.allow_insecure;

    let fixed_url = config.cluster_url.to_string().replace("0.0.0.0", "127.0.0.1");
    if let Ok(uri) = Uri::from_str(&fixed_url) {
        config.cluster_url = uri;
    }

    Ok(Client::try_from(config)?)
}

/// Returns provided context (or default one if `None` specified).
fn get_context_internal(kubeconfig: &Kubeconfig, context: Option<&str>) -> Option<String> {
    let Some(context) = context else {
        return kubeconfig.current_context.as_ref().map(String::from);
    };

    let context = kubeconfig.contexts.iter().find(|c| c.name == context);
    context.map(|context| context.name.clone())
}

/// Returns kube config.
async fn get_kubeconfig(kubeconfig_path: Option<&str>) -> Result<(Kubeconfig, Option<String>), ClientError> {
    let path = resolve_kubeconfig_path(kubeconfig_path)?;
    let path_result = if kubeconfig_path.is_some() {
        Some(path.to_str().unwrap_or_default().to_string())
    } else {
        None
    };
    let mut file = File::open(path).await?;

    let mut kubeconfig_str = String::new();
    file.read_to_string(&mut kubeconfig_str).await?;

    Ok((Kubeconfig::from_yaml(&kubeconfig_str)?, path_result))
}
