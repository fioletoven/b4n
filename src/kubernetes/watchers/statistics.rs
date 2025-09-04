use kube::{ResourceExt, api::DynamicObject};
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
};

use crate::{
    core::DiscoveryList,
    kubernetes::{
        Kind, client::KubernetesClient, metrics::Metrics, resources::PODS, utils::get_resource, watchers::observer::BgObserver,
    },
    ui::widgets::FooterTx,
};

pub type SharedStatistics = Rc<RefCell<Statistics>>;

#[derive(Debug)]
pub struct NodeStats {
    pub metrics: Option<Metrics>,
    pub pods: Vec<PodStats>,
}

#[derive(Debug)]
pub struct PodStats {
    pub name: String,
    pub namespace: String,
    pub metrics: Option<Metrics>,
    pub containers: Vec<ContainerStats>,
}

#[derive(Debug)]
pub struct ContainerStats {
    pub name: String,
    pub metrics: Option<Metrics>,
}

#[derive(Default)]
struct PodData {
    node_name: String,
    name: String,
    namespace: String,
    containers: HashMap<String, Option<Metrics>>,
}

impl From<&DynamicObject> for PodData {
    fn from(value: &DynamicObject) -> Self {
        Self {
            node_name: value.data["spec"]["nodeName"].as_str().map(String::from).unwrap_or_default(),
            name: value.name_any(),
            namespace: value.namespace().unwrap_or_default(),
            containers: get_containers(value),
        }
    }
}

pub struct Statistics {
    pub has_metrics: bool,
    data: HashMap<String, NodeStats>,
}

impl Statistics {
    pub fn all_nodes_count(&self) -> usize {
        self.data.len()
    }

    pub fn all_pods_count(&self) -> usize {
        self.data.values().map(|n| n.pods.len()).sum()
    }

    pub fn all_containers_count(&self) -> usize {
        self.data
            .values()
            .map(|n| n.pods.iter().map(|p| p.containers.len()).sum::<usize>())
            .sum()
    }

    pub fn pods_count(&self, node_name: &str) -> usize {
        self.data.get(node_name).map(|node| node.pods.len()).unwrap_or_default()
    }

    pub fn containers_count(&self, node_name: &str) -> usize {
        self.data
            .get(node_name)
            .map(|node| node.pods.iter().map(|p| p.containers.len()).sum())
            .unwrap_or_default()
    }
}

pub struct BgStatistics {
    stats: SharedStatistics,
    pods: BgObserver,
    data: HashMap<String, PodData>,
    footer_tx: FooterTx,
    is_dirty: bool,
    has_metrics: bool,
}

impl BgStatistics {
    pub fn new(footer_tx: FooterTx) -> Self {
        Self {
            stats: Rc::new(RefCell::new(Statistics {
                data: HashMap::new(),
                has_metrics: false,
            })),
            pods: BgObserver::new(footer_tx.clone()),
            data: HashMap::new(),
            footer_tx,
            is_dirty: false,
            has_metrics: false,
        }
    }

    pub fn start(&mut self, client: &KubernetesClient, discovery_list: Option<&DiscoveryList>) {
        if let Some(discovery) = get_resource(discovery_list, &Kind::new(PODS, ""))
            && self.pods.start(client, (&discovery.0).into(), Some(discovery)).is_err()
        {
            self.footer_tx.show_error("Cannot run statistics task", 0);
        }
    }

    pub fn update_statistics(&mut self) {
        self.is_dirty = false;
        if self.pods.is_ready() {
            while let Some(result) = self.pods.try_next() {
                match *result {
                    super::ObserverResult::Apply(result) => self.add_pod_data(&result),
                    super::ObserverResult::Delete(result) => self.del_pod_data(&result),
                    _ => (),
                }
            }
        }

        if self.is_dirty {
            self.recalculate_statistics();
        }
    }

    pub fn share(&self) -> SharedStatistics {
        self.stats.clone()
    }

    fn recalculate_statistics(&mut self) {
        let mut new_stats = self
            .data
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

        for pod in self.data.values() {
            if let Some(node) = new_stats.get_mut(&pod.node_name) {
                node.pods.push(PodStats {
                    name: pod.name.clone(),
                    namespace: pod.namespace.clone(),
                    metrics: if self.has_metrics {
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
                });
            }
        }

        if self.has_metrics {
            for node in new_stats.values_mut() {
                node.metrics = Some(node.pods.iter().filter_map(|p| p.metrics).sum());
            }
        }

        self.stats.replace(Statistics {
            data: new_stats,
            has_metrics: self.has_metrics,
        });
    }

    fn add_pod_data(&mut self, resource: &DynamicObject) {
        let uid = get_uid(resource);

        self.data
            .entry(uid)
            .and_modify(|pod| update_containers(&mut pod.containers, resource))
            .or_insert_with(|| resource.into());

        self.is_dirty = true;
    }

    fn del_pod_data(&mut self, resource: &DynamicObject) {
        self.data.remove(&get_uid(resource));
        self.is_dirty = true;
    }
}

fn get_uid(result: &DynamicObject) -> String {
    format!("{}.{}", result.name_any(), result.namespace().unwrap_or_default())
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

fn update_containers(containers: &mut HashMap<String, Option<Metrics>>, resource: &DynamicObject) {
    let Some(new_containers) = resource.data["spec"]["containers"].as_array() else {
        containers.clear();
        return;
    };

    let new_names = new_containers
        .iter()
        .filter_map(|c| c["name"].as_str())
        .collect::<HashSet<_>>();

    containers.retain(|c, _| new_names.contains(&c.as_str()));

    for container in new_containers {
        if let Some(name) = container["name"].as_str()
            && !containers.contains_key(name)
        {
            containers.insert(name.to_owned(), None);
        }
    }
}
