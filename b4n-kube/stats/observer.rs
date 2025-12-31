use b4n_common::{DEFAULT_ERROR_DURATION, NotificationSink};
use kube::{ResourceExt, api::DynamicObject};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use tokio::runtime::Handle;

use crate::client::KubernetesClient;
use crate::stats::Metrics;
use crate::utils::get_resource;
use crate::{BgObserver, DiscoveryList, Kind, NODES, ObserverResult, PODS};

pub type SharedStatistics = Rc<RefCell<Statistics>>;

/// Holds `node` statistics.
#[derive(Debug)]
pub struct NodeStats {
    pub metrics: Option<Metrics>,
    pub pods: Vec<PodStats>,
}

/// Holds `pod` statistics.
#[derive(Debug)]
pub struct PodStats {
    pub name: String,
    pub namespace: String,
    pub metrics: Option<Metrics>,
    pub containers: Vec<ContainerStats>,
}

impl PodStats {
    /// Creates new [`PodStats`] instance from [`PodData`] reference.
    fn from(pod: &PodData, has_metrics: bool) -> Self {
        PodStats {
            name: pod.name.clone(),
            namespace: pod.namespace.clone(),
            metrics: if has_metrics {
                Some(pod.containers.values().filter_map(|c| *c).sum())
            } else {
                None
            },
            containers: pod
                .containers
                .iter()
                .map(|(name, metrics)| ContainerStats {
                    name: name.clone(),
                    metrics: *metrics,
                })
                .collect(),
        }
    }

    /// Returns specified container from the pod statistics.
    pub fn container(&self, container_name: &str) -> Option<&ContainerStats> {
        self.containers.iter().find(|c| c.name == container_name)
    }
}

/// Holds `container` statistics.
#[derive(Debug)]
pub struct ContainerStats {
    pub name: String,
    pub metrics: Option<Metrics>,
}

/// Holds all statistics for the Kubernetes cluster.
#[derive(Debug)]
pub struct Statistics {
    pub generation: u16,
    pub has_metrics: bool,
    data: HashMap<String, NodeStats>,
}

impl Statistics {
    /// Returns number of nodes in the Kubernetes cluster.
    pub fn all_nodes_count(&self) -> usize {
        self.data.len()
    }

    /// Returns number of pods in the Kubernetes cluster.
    pub fn all_pods_count(&self) -> usize {
        self.data.values().map(|n| n.pods.len()).sum()
    }

    /// Returns number of containers in the Kubernetes cluster.
    pub fn all_containers_count(&self) -> usize {
        self.data
            .values()
            .map(|n| n.pods.iter().map(|p| p.containers.len()).sum::<usize>())
            .sum()
    }

    /// Returns number of pods in the Kubernetes node.
    pub fn pods_count(&self, node_name: &str) -> usize {
        self.data.get(node_name).map(|node| node.pods.len()).unwrap_or_default()
    }

    /// Returns number of containers in the Kubernetes node.
    pub fn containers_count(&self, node_name: &str) -> usize {
        self.data
            .get(node_name)
            .map(|node| node.pods.iter().map(|p| p.containers.len()).sum())
            .unwrap_or_default()
    }

    /// Returns specified node from the statistics.
    pub fn node(&self, node_name: &str) -> Option<&NodeStats> {
        self.data.get(node_name)
    }

    /// Returns CPU usage for the Kubernetes node.
    pub fn node_cpu(&self, node_name: &str) -> u64 {
        self.data
            .get(node_name)
            .and_then(|node| node.metrics)
            .map(|metrics| metrics.cpu.value)
            .unwrap_or_default()
    }

    /// Returns Memory usage for the Kubernetes node.
    pub fn node_memory(&self, node_name: &str) -> u64 {
        self.data
            .get(node_name)
            .and_then(|node| node.metrics)
            .map(|metrics| metrics.memory.value)
            .unwrap_or_default()
    }

    /// Returns specified pod from the statistics.
    pub fn pod(&self, node_name: &str, pod_name: &str, pod_namespace: &str) -> Option<&PodStats> {
        self.data
            .get(node_name)
            .and_then(|n| n.pods.iter().find(|p| p.name == pod_name && p.namespace == pod_namespace))
    }
}

#[derive(Default, Debug)]
struct PodData {
    node_name: String,
    name: String,
    namespace: String,
    containers: HashMap<String, Option<Metrics>>,
}

impl From<&DynamicObject> for PodData {
    fn from(value: &DynamicObject) -> Self {
        Self {
            node_name: get_node_name(value),
            name: value.name_any(),
            namespace: value.namespace().unwrap_or_default(),
            containers: get_containers(value),
        }
    }
}

/// Collects and stores pod and node metrics for the Kubernetes cluster.\
/// **Note** that it runs up to 3 background observers for tracking changes.
pub struct BgStatistics {
    stats: SharedStatistics,
    pods: BgObserver,
    pods_metrics: BgObserver,
    nodes_metrics: BgObserver,
    pod_data: HashMap<String, PodData>,
    node_data: HashMap<String, Option<Metrics>>,
    footer_tx: NotificationSink,
    is_dirty: bool,
    has_metrics: bool,
}

impl BgStatistics {
    /// Creates new [`BgStatistics`] instance.
    pub fn new(runtime: Handle, footer_tx: NotificationSink) -> Self {
        Self {
            stats: Rc::new(RefCell::new(Statistics {
                generation: 0,
                data: HashMap::new(),
                has_metrics: false,
            })),
            pods: BgObserver::new(runtime.clone(), None),
            pods_metrics: BgObserver::new(runtime.clone(), None),
            nodes_metrics: BgObserver::new(runtime, None),
            pod_data: HashMap::new(),
            node_data: HashMap::new(),
            footer_tx,
            is_dirty: false,
            has_metrics: false,
        }
    }

    /// Starts new [`BgStatistics`] task.\
    /// **Note** that it stops the old tasks if any is running.
    pub fn start(&mut self, client: &KubernetesClient, discovery_list: Option<&DiscoveryList>) {
        self.stop();
        self.has_metrics = false;

        if let Some(discovery) = get_resource(discovery_list, &Kind::new(PODS, "", ""))
            && self.pods.start(client, (&discovery.0).into(), Some(discovery), true).is_err()
        {
            self.footer_tx
                .show_error("Cannot run statistics task", DEFAULT_ERROR_DURATION);
        }

        if let Some(discovery) = get_resource(discovery_list, &Kind::new("pods", "metrics.k8s.io", "")) {
            self.has_metrics = self
                .pods_metrics
                .start(client, (&discovery.0).into(), Some(discovery), true)
                .is_ok();
        }

        if let Some(discovery) = get_resource(discovery_list, &Kind::new(NODES, "metrics.k8s.io", "")) {
            self.has_metrics = self
                .nodes_metrics
                .start(client, (&discovery.0).into(), Some(discovery), true)
                .is_ok();
        }
    }

    /// Cancels [`BgStatistics`] task.
    pub fn cancel(&mut self) {
        self.pods.cancel();
        self.pods_metrics.cancel();
        self.nodes_metrics.cancel();
    }

    /// Cancels [`BgStatistics`] task and waits until it is finished.
    pub fn stop(&mut self) {
        self.cancel();

        self.pods.stop();
        self.pods_metrics.stop();
        self.nodes_metrics.stop();
    }

    /// Updates cached statistics object with new data from observers.
    pub fn update_statistics(&mut self) {
        self.is_dirty = false;
        if self.pods.is_ready() {
            while let Some(result) = self.pods.try_next() {
                match *result {
                    ObserverResult::Apply(result) => self.add_pod_data(&result),
                    ObserverResult::Delete(result) => self.del_pod_data(&result),
                    _ => (),
                }
            }

            while let Some(result) = self.pods_metrics.try_next() {
                if let ObserverResult::Apply(result) = *result {
                    self.add_pod_metrics(&result);
                }
            }

            while let Some(result) = self.nodes_metrics.try_next() {
                if let ObserverResult::Apply(result) = *result {
                    self.add_node_metrics(&result);
                }
            }
        }

        if self.is_dirty {
            self.recalculate_statistics();
        }
    }

    /// Returns cloned [`SharedStatistics`] object.
    pub fn share(&self) -> SharedStatistics {
        self.stats.clone()
    }

    /// Returns `true` if pods statistics observer has an error.
    pub fn has_error(&self) -> bool {
        self.pods.has_error()
    }

    /// Returns `true` if pods statistics observer has a connection error.
    pub fn has_connection_error(&self) -> bool {
        self.pods.has_connection_error()
    }

    fn recalculate_statistics(&mut self) {
        let mut new_stats = self
            .pod_data
            .values()
            .map(|pod| {
                (
                    pod.node_name.clone(),
                    NodeStats {
                        metrics: None,
                        pods: Vec::new(),
                    },
                )
            })
            .collect::<HashMap<_, _>>();

        for pod in self.pod_data.values() {
            if let Some(node) = new_stats.get_mut(&pod.node_name) {
                node.pods.push(PodStats::from(pod, self.has_metrics));
            }
        }

        if self.has_metrics {
            for (name, node) in &mut new_stats {
                if let Some(&metrics) = self.node_data.get(name) {
                    node.metrics = metrics;
                }
            }
        }

        let generation = self.stats.borrow().generation.wrapping_add(1);
        self.stats.replace(Statistics {
            generation,
            has_metrics: self.has_metrics,
            data: new_stats,
        });
    }

    fn add_pod_data(&mut self, resource: &DynamicObject) {
        let uid = get_uid(resource);

        self.pod_data
            .entry(uid)
            .and_modify(|pod| update_pod(pod, resource))
            .or_insert_with(|| resource.into());

        self.is_dirty = true;
    }

    fn del_pod_data(&mut self, resource: &DynamicObject) {
        self.pod_data.remove(&get_uid(resource));
        self.is_dirty = true;
    }

    fn add_pod_metrics(&mut self, resource: &DynamicObject) {
        let uid = get_uid(resource);
        if let Some(pod) = self.pod_data.get_mut(&uid)
            && let Some(containers) = resource.data["containers"].as_array()
        {
            for container in containers {
                let name = container["name"].as_str().unwrap_or_default();
                if let Some(metrics) = pod.containers.get_mut(name) {
                    *metrics = Metrics::try_from(container).ok();
                }
            }

            self.is_dirty = true;
        }
    }

    fn add_node_metrics(&mut self, resource: &DynamicObject) {
        let name = resource.name_any();
        self.node_data
            .entry(name)
            .and_modify(|metrics| *metrics = Metrics::try_from(&resource.data).ok())
            .or_insert_with(|| Metrics::try_from(&resource.data).ok());
    }
}

fn get_node_name(pod: &DynamicObject) -> String {
    pod.data["spec"]["nodeName"].as_str().map(String::from).unwrap_or_default()
}

fn get_uid(resource: &DynamicObject) -> String {
    format!("{}.{}", resource.name_any(), resource.namespace().unwrap_or_default())
}

fn get_containers(resource: &DynamicObject) -> HashMap<String, Option<Metrics>> {
    resource.data["spec"]["containers"]
        .as_array()
        .map_or_else(HashMap::new, |containers| {
            containers
                .iter()
                .filter_map(|container| container["name"].as_str().map(|name| (name.to_string(), None)))
                .collect()
        })
}

fn update_pod(pod: &mut PodData, resource: &DynamicObject) {
    if pod.node_name.is_empty() {
        pod.node_name = get_node_name(resource);
    }

    let Some(new_containers) = resource.data["spec"]["containers"].as_array() else {
        pod.containers.clear();
        return;
    };

    let new_names = new_containers
        .iter()
        .filter_map(|c| c["name"].as_str())
        .collect::<HashSet<_>>();

    pod.containers.retain(|c, _| new_names.contains(&c.as_str()));

    for container in new_containers {
        if let Some(name) = container["name"].as_str()
            && !pod.containers.contains_key(name)
        {
            pod.containers.insert(name.to_owned(), None);
        }
    }
}
