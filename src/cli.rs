use clap::Parser;

use crate::kubernetes::ALL_NAMESPACES;

/// Simple program to list resources in kubernetes.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Path to the kube config file.
    #[arg(long)]
    pub kube_config: Option<String>,

    /// Context to use, defined in kube config
    #[arg(long)]
    pub context: Option<String>,

    /// Kubernetes resource to list
    #[arg()]
    pub resource: Option<String>,

    /// Kubernetes namespace for the resource to list
    #[arg(long, short)]
    pub namespace: Option<String>,

    /// List resource in all namespaces
    #[arg(long)]
    pub all_namespaces: bool,
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
