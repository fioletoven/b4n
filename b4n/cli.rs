use b4n_kube::ALL_NAMESPACES;
use clap::Parser;

/// b4n is an interactive TUI for managing Kubernetes clusters.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Path to the kubeconfig file (defaults to $HOME/.kube/config).
    #[arg(long, env = "KUBECONFIG")]
    pub kube_config: Option<String>,

    /// Context to use from the kubeconfig file.
    #[arg(long)]
    pub context: Option<String>,

    /// Kubernetes resource kind to show (e.g. pods, deployments, services).
    #[arg()]
    pub resource: Option<String>,

    /// Namespace to focus on at startup.
    #[arg(long, short)]
    pub namespace: Option<String>,

    /// Start with cluster-wide view (all namespaces).
    #[arg(long, short = 'A')]
    pub all_namespaces: bool,

    /// Skip TLS certificate verification (insecure).
    #[arg(long)]
    pub insecure: bool,
}

impl Args {
    /// Returns context or default if context is `None`.
    pub fn context<'a>(&'a self, default: Option<&'a str>) -> Option<&'a str> {
        if self.context.is_some() {
            self.context.as_deref()
        } else {
            default
        }
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
