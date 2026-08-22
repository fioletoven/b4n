use b4n_kube::ALL_NAMESPACES;
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

    /// Start with cluster-wide view (all namespaces).
    #[arg(long, short = 'A')]
    pub all_namespaces: bool,

    /// Path to the kubeconfig file (defaults to $HOME/.kube/config).
    #[arg(long, env = "KUBECONFIG")]
    pub kube_config: Option<String>,

    /// Context to use from the kubeconfig file.
    #[arg(long)]
    pub context: Option<String>,

    /// Cluster to use from the kubeconfig file.
    #[arg(long)]
    pub cluster: Option<String>,

    /// User to use from the kubeconfig file.
    #[arg(long)]
    pub user: Option<String>,

    /// Skip TLS certificate verification (insecure).
    #[arg(long)]
    pub insecure: bool,

    /// Print configuration paths used by the application.
    #[arg(long)]
    pub show_dirs: bool,
}

impl Args {
    /// Returns context or `last_used` if context is `None` and cluster or user is not provided.
    pub fn context<'a>(&'a self, last_used: Option<&'a str>) -> Option<&'a str> {
        self.context.as_deref().or_else(|| {
            let allow_last_used = self.cluster.is_none() && self.user.is_none();
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
