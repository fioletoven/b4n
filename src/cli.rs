use clap::Parser;

use crate::kubernetes::ALL_NAMESPACES;

/// Simple program to list resources in kubernetes
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Context to use, defined in kube config
    #[arg(long)]
    pub context: Option<String>,

    /// Kubernetes resource to list
    #[arg(default_value = "pods")]
    pub resource: String,

    /// Kubernetes namespace for the resource to list
    #[arg(long, short, default_value = "kube-system")]
    pub namespace: String,

    /// List resource in all namespaces
    #[arg(long)]
    pub all_namespaces: bool,
}

impl Args {
    /// Returns the namespace option respecting `--all-namespaces` switch
    pub fn namespace(&self) -> Option<String> {
        if self.all_namespaces || self.namespace == ALL_NAMESPACES {
            return None;
        }

        Some(self.namespace.clone())
    }
}
