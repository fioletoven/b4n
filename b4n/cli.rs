use b4n_kube::{ALL_NAMESPACES, client::ClientOptions};
use clap::Parser;

/// b4n is an interactive TUI for managing Kubernetes clusters.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Kubernetes resource kind to show (e.g. pods, deployments, services).
    #[arg()]
    pub resource: Option<String>,

    /// Namespace to focus on at startup.
    #[arg(long, short)]
    pub namespace: Option<String>,

    /// Show resources across all namespaces.
    #[arg(long, short = 'A')]
    pub all_namespaces: bool,

    /// Path to the kubeconfig file (defaults to ~/.kube/config).
    #[arg(long, alias = "kube-config", env = "KUBECONFIG")]
    pub kubeconfig: Option<String>,

    /// Context to use from the kubeconfig file.
    #[arg(long)]
    pub context: Option<String>,

    /// Cluster to use from the kubeconfig file.
    #[arg(long)]
    pub cluster: Option<String>,

    /// User to use from the kubeconfig file.
    #[arg(long)]
    pub user: Option<String>,

    /// User to impersonate during the session.
    #[arg(long = "as")]
    pub as_user: Option<String>,

    /// Group to impersonate during the session.
    #[arg(long, requires = "as_user")]
    pub as_group: Vec<String>,

    /// Path to a client certificate file for TLS authentication.
    #[arg(long, requires = "client_key")]
    pub client_cert: Option<String>,

    /// Path to a client key file for TLS authentication.
    #[arg(long, requires = "client_cert")]
    pub client_key: Option<String>,

    /// Path to a CA file for server TLS certificate verification.
    #[arg(long = "ca")]
    pub certificate_authority: Option<String>,

    /// Skip server TLS certificate verification (unsafe).
    #[arg(long)]
    pub insecure: bool,

    /// Print configuration paths used by the application.
    #[arg(long)]
    pub show_dirs: bool,
}

impl Args {
    /// Returns context name or `last_used` if:
    /// - context is `None`,
    /// - cluster, user or impersonation is not provided.
    pub fn context<'a>(&'a self, last_used: Option<&'a str>) -> Option<&'a str> {
        self.context.as_deref().or_else(|| {
            let allow_last_used =
                self.cluster.is_none() && self.user.is_none() && self.as_user.is_none() && self.as_group.is_empty();
            if allow_last_used { last_used } else { None }
        })
    }

    /// Returns the namespace option respecting `--all-namespaces` switch.
    pub fn namespace<'a>(&'a self, default: Option<&'a str>) -> Option<&'a str> {
        if self.all_namespaces {
            return None;
        }

        let namespace = if self.namespace.is_some() {
            self.namespace.as_deref()
        } else {
            default
        };

        if namespace.is_some_and(|n| n == ALL_NAMESPACES) {
            None
        } else {
            namespace
        }
    }

    // Returns resource kind or default if resource is `None`.
    pub fn kind<'a>(&'a self, default: Option<&'a str>) -> Option<&'a str> {
        if self.resource.is_some() {
            self.resource.as_deref()
        } else {
            default
        }
    }
}

impl From<&Args> for ClientOptions {
    fn from(value: &Args) -> Self {
        Self {
            cluster: value.cluster.clone(),
            user: value.user.clone(),
            as_user: value.as_user.clone(),
            as_groups: (!value.as_group.is_empty()).then(|| value.as_group.clone()),
            client_cert: value.client_cert.clone(),
            client_key: value.client_key.clone(),
            certificate_authority: value.certificate_authority.clone(),
            allow_insecure: value.insecure,
        }
    }
}
