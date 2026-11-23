use b4n_common::{DEFAULT_ERROR_DURATION, DEFAULT_MESSAGE_DURATION, NotificationSink};
use b4n_kube::ResourceRef;
use k8s_openapi::api::batch::v1::{CronJob, Job};
use kube::api::{ObjectMeta, PostParams};
use kube::{Api, Client, Resource};

use crate::commands::CommandResult;

/// Command that creates a Job from a CronJob's template.
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

    /// Creates a Job from a CronJob's template.
    pub async fn execute(self) -> Option<CommandResult> {
        match create_job(self.client, &self.resource).await {
            Ok(job) => {
                let msg = format!(
                    "Job '{}' created from CronJob '{}' in namespace '{}'",
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

async fn create_job(client: Client, resource: &ResourceRef) -> Result<Job, String> {
    let name = resource
        .name
        .as_deref()
        .filter(|n| !n.is_empty())
        .ok_or("CronJob name is not provided")?;
    let namespace = resource.namespace.as_option().ok_or("CronJob namespace is not provided")?;

    let cronjobs: Api<CronJob> = Api::namespaced(client, namespace);
    let cronjob = cronjobs.get(name).await.map_err(|error| error.to_string())?;

    let jobs: Api<Job> = Api::namespaced(cronjobs.into_client(), namespace);

    jobs.create(&PostParams::default(), &(build_job(cronjob, name)?))
        .await
        .map_err(|error| error.to_string())
}

fn build_job(cronjob: CronJob, name: &str) -> Result<Job, &'static str> {
    let owner = cronjob.controller_owner_ref(&()).map(|mut owner| {
        owner.block_owner_deletion = None;
        owner
    });
    let template = cronjob.spec.job_template;

    let spec = template.spec.ok_or("CronJob has no Job template spec")?;
    let metadata = template.metadata.unwrap_or_default();

    let mut annotations = metadata.annotations.unwrap_or_default();
    annotations.insert("cronjob.kubernetes.io/instantiate".to_owned(), "manual".to_owned());

    Ok(Job {
        metadata: ObjectMeta {
            generate_name: Some(format!("{name}-")),
            labels: metadata.labels,
            annotations: Some(annotations),
            owner_references: owner.map(|owner| vec![owner]),
            ..Default::default()
        },
        spec: Some(spec),
        ..Default::default()
    })
}
