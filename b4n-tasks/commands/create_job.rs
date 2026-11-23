use b4n_common::{DEFAULT_ERROR_DURATION, DEFAULT_MESSAGE_DURATION, NotificationSink};
use b4n_kube::ResourceRef;
use k8s_openapi::api::batch::v1::{CronJob, Job};
use k8s_openapi::serde_json::{Value, json};
use kube::api::{ApiResource, DynamicObject, ObjectMeta, PostParams};
use kube::{Api, Client, Resource};
use std::collections::BTreeMap;

use crate::commands::CommandResult;

/// Command that creates a `Job` from a `CronJobs` template.
pub struct CreateJobCommand {
    client: Client,
    resource: ResourceRef,
    sink: NotificationSink,
}

impl CreateJobCommand {
    /// Creates new [`CreateJobCommand`] instance.
    pub fn new(client: Client, resource: ResourceRef, sink: NotificationSink) -> Self {
        Self { client, resource, sink }
    }

    /// Creates a `Job` from a `CronJobs` template.
    pub async fn execute(self) -> Option<CommandResult> {
        match create_job(self.client, &self.resource).await {
            Ok(job) => {
                let msg = format!(
                    "Job '{}' created from '{}' in namespace '{}'",
                    job.metadata.name.as_deref().unwrap_or_default(),
                    self.resource.name.as_deref().unwrap_or_default(),
                    self.resource.namespace.as_str()
                );
                tracing::info!("{}", msg);
                self.sink.show_info(msg, DEFAULT_MESSAGE_DURATION);
            },
            Err(error) => {
                let msg = format!(
                    "Cannot create Job from CronJob '{}' in namespace '{}': {error}",
                    self.resource.name.as_deref().unwrap_or_default(),
                    self.resource.namespace.as_str()
                );
                tracing::error!("{}", msg);
                self.sink.show_error(msg, DEFAULT_ERROR_DURATION);
            },
        }

        None
    }
}

async fn create_job(client: Client, resource: &ResourceRef) -> Result<DynamicObject, String> {
    let name = resource
        .name
        .as_deref()
        .filter(|n| !n.is_empty())
        .ok_or("CronJob name is not provided")?;
    let namespace = resource.namespace.as_option().ok_or("CronJob namespace is not provided")?;

    let cronjob_ar = ApiResource::erase::<CronJob>(&());
    let cronjobs: Api<DynamicObject> = Api::namespaced_with(client, namespace, &cronjob_ar);
    let cronjob = cronjobs.get(name).await.map_err(|error| error.to_string())?;

    let job_ar = ApiResource::erase::<Job>(&());
    let jobs: Api<DynamicObject> = Api::namespaced_with(cronjobs.into_client(), namespace, &job_ar);

    jobs.create(&PostParams::default(), &(build_job(&cronjob, &cronjob_ar, &job_ar, name)?))
        .await
        .map_err(|error| error.to_string())
}

fn build_job(
    cronjob: &DynamicObject,
    cronjob_ar: &ApiResource,
    job_ar: &ApiResource,
    name: &str,
) -> Result<DynamicObject, &'static str> {
    let template = &cronjob.data["spec"]["jobTemplate"];
    let spec = template.get("spec").cloned().ok_or("CronJob has no Job template spec")?;

    let labels = get_map(template, "labels");
    let mut annotations = get_map(template, "annotations");
    annotations.insert("cronjob.kubernetes.io/instantiate".to_owned(), "manual".to_owned());

    let owner = cronjob.controller_owner_ref(cronjob_ar).map(|mut owner| {
        owner.block_owner_deletion = None;
        owner
    });

    let mut job = DynamicObject::new("", job_ar);
    job.metadata = ObjectMeta {
        generate_name: Some(format!("{name}-")),
        labels: (!labels.is_empty()).then_some(labels),
        annotations: Some(annotations),
        owner_references: owner.map(|owner| vec![owner]),
        ..Default::default()
    };
    job.data = json!({ "spec": spec });

    Ok(job)
}

fn get_map(template: &Value, key: &str) -> BTreeMap<String, String> {
    let Some(map) = template["metadata"][key].as_object() else {
        return BTreeMap::new();
    };

    map.iter()
        .filter_map(|(k, v)| v.as_str().map(|v| (k.clone(), v.to_owned())))
        .collect()
}
