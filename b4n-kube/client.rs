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

    /// Kubeconfig file not found.
    #[error("kubeconfig file not found")]
    KubeConfigNotFound,

    /// Context not found in kubeconfig.
    #[error("Kube context '{0}' not found in configuration.")]
    ContextNotFound(String),

    /// Cluster not found in kubeconfig.
    #[error("Kube cluster '{0}' not found in configuration.")]
    ClusterNotFound(String),

    /// User not found in kubeconfig.
    #[error("Kube user '{0}' not found in configuration.")]
    UserNotFound(String),

    /// Certificate file not found.
    #[error("Certificate path '{0}' not found.")]
    CertificateNotFound(String),

    /// Failed to read kube configuration.
    #[error("cannot read kubeconfig: {0}")]
    IoError(#[from] std::io::Error),

    /// Failed to process kube configuration.
    #[error("cannot build kubeconfig: {0}")]
    KubeconfigError(#[from] kube::config::KubeconfigError),

    /// Failed to build kubernetes client.
    #[error("cannot create client: {0}")]
    KubeError(#[from] kube::Error),

    /// Cannot join client creation task.
    #[error("create client task join error: {0}")]
    TaskJoin(#[from] tokio::task::JoinError),
}

/// Options for the Kubernetes client.
#[derive(Clone)]
pub struct ClientOptions {
    pub cluster: Option<String>,
    pub user: Option<String>,
    pub as_user: Option<String>,
    pub as_groups: Option<Vec<String>>,

    pub client_cert: Option<String>,
    pub client_key: Option<String>,
    pub certificate_authority: Option<String>,
    pub allow_insecure: bool,
}

impl ClientOptions {
    /// Creates new [`ClientOptions`] instance.
    pub fn new(allow_insecure: bool) -> Self {
        Self {
            cluster: None,
            user: None,
            as_user: None,
            as_groups: None,
            client_cert: None,
            client_key: None,
            certificate_authority: None,
            allow_insecure,
        }
    }

    /// Returns `true` if options have overrides for cluster, user or impersonation.
    pub fn has_overrides(&self) -> bool {
        self.cluster.is_some()
            || self.user.is_some()
            || self.as_user.is_some()
            || self.as_groups.as_ref().is_some_and(|g| !g.is_empty())
    }
}

impl std::fmt::Display for ClientOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut sep = "";

        if let Some(cluster) = &self.cluster {
            write!(f, "cluster: {cluster}")?;
            sep = ", ";
        }

        if let Some(user) = &self.user {
            write!(f, "{sep}user: {user}")?;
            sep = ", ";
        }

        if let Some(as_user) = &self.as_user {
            write!(f, "{sep}as: {as_user}")?;
            sep = ", ";
        }

        if let Some(as_groups) = &self.as_groups
            && !as_groups.is_empty()
        {
            write!(f, "{sep}as-groups: ")?;
            sep = "";
            for group in as_groups {
                write!(f, "{sep}{group}")?;
                sep = ", ";
            }
        }

        Ok(())
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
    client: Client,
    kubeconfig_path: Option<String>,
    context: String,
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

    /// Returns path to kubeconfig used to create this client.
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

/// Validates all provided certificate paths if they exist.
pub fn validate_certificate_paths(paths: &[Option<&str>]) -> Result<(), ClientError> {
    paths
        .iter()
        .flatten()
        .map(std::path::Path::new)
        .filter(|path| !path.exists() || !path.is_file())
        .try_for_each(|path| Err(ClientError::CertificateNotFound(path.display().to_string())))
}

/// Validates provided configuration and returns matching context from the kubeconfig.\
/// **Note** that it can `fallback_to_default` if the provided context is not found in kubeconfig.
pub async fn validate_and_resolve_context(
    kubeconfig_path: Option<&str>,
    context: Option<&str>,
    cluster: Option<&str>,
    user: Option<&str>,
    fallback_to_default: bool,
) -> Result<(ContextInfo, Option<String>), ClientError> {
    let (kubeconfig, kubeconfig_path) = get_kubeconfig(kubeconfig_path).await?;

    if let Some(cluster_name) = cluster
        && kubeconfig.clusters.iter().find(|c| c.name == cluster_name).is_none()
    {
        return Err(ClientError::ClusterNotFound(cluster_name.to_string()));
    }

    if let Some(user_name) = user
        && kubeconfig.auth_infos.iter().find(|c| c.name == user_name).is_none()
    {
        return Err(ClientError::UserNotFound(user_name.to_string()));
    }

    if let Some(context_name) = context
        && let Some(context) = kubeconfig.contexts.iter().find(|c| c.name == context_name)
    {
        Ok((context.into(), kubeconfig_path))
    } else if fallback_to_default
        && let Some(context_name) = kubeconfig.current_context.as_deref()
        && let Some(context) = kubeconfig.contexts.iter().find(|c| c.name == context_name)
    {
        Ok((context.into(), kubeconfig_path))
    } else {
        Err(ClientError::ContextNotFound(context.unwrap_or("default").to_string()))
    }
}

/// Returns contexts from the kubeconfig.
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
        Err(ClientError::ContextNotFound(context.unwrap_or("default").to_string()))
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
    config.auth_info.impersonate = options.as_user;
    config.auth_info.impersonate_groups = options.as_groups;
    config.auth_info.client_certificate = options.client_cert;
    config.auth_info.client_key = options.client_key;
    config.accept_invalid_certs = options.allow_insecure;
    config.root_cert_file = options.certificate_authority.as_ref().map(PathBuf::from);

    let fixed_url = config.cluster_url.to_string().replace("0.0.0.0", "127.0.0.1");
    if let Ok(uri) = Uri::from_str(&fixed_url) {
        config.cluster_url = uri;
    }

    Ok(tokio::task::spawn_blocking(move || Client::try_from(config)).await??)
}

/// Returns provided context (or default one if `None` specified).
fn get_context_internal(kubeconfig: &Kubeconfig, context: Option<&str>) -> Option<String> {
    let Some(context) = context else {
        return kubeconfig.current_context.as_ref().map(String::from);
    };

    let context = kubeconfig.contexts.iter().find(|c| c.name == context);
    context.map(|context| context.name.clone())
}

/// Returns kubeconfig.
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
