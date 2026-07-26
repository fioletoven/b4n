use b4n_kube::{ResourceRef, files::TransferContext};
use k8s_openapi::api::core::v1::Pod;
use kube::{Api, Client, api::AttachParams};
use std::path::{Path, PathBuf};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::runtime::Handle;

use crate::commands::CommandResult;

/// Possible file transfer errors.
#[derive(thiserror::Error, Debug)]
pub enum TransferFileError {
    #[error("failed to create tar buffer: {0}")]
    TarIoError(#[from] std::io::Error),

    #[error("kube client error: {0}")]
    KubeError(#[from] kube::Error),

    #[error("failed to wait for remote process: {0}")]
    RemoteProcessError(String),

    #[error("task join error: {0}")]
    JoinError(#[from] tokio::task::JoinError),

    #[error("invalid path: {0}")]
    InvalidPath(PathBuf),

    #[error("missing stdout from attached process")]
    MissingStdout,

    #[error("missing stdin from attached process")]
    MissingStdin,

    #[error("missing stderr from attached process")]
    MissingStderr,
}

/// Result from the file transfer command.
pub struct TransferFileResult {
    pub is_download: bool,
    pub pod: String,
    pub container: String,
    pub file: String,
}

/// File transfer command that sends/downloads file from/to a pod's container.
pub struct TransferFileCommand {
    runtime: Handle,
    resource: ResourceRef,
    context: TransferContext,
    client: Client,
}

impl TransferFileCommand {
    /// Creates new file transfer command.
    pub fn new(runtime: Handle, resource: ResourceRef, context: TransferContext, client: Client) -> Self {
        Self {
            runtime,
            resource,
            context,
            client,
        }
    }

    pub async fn execute(self) -> Option<CommandResult> {
        let pods: Api<Pod> = Api::namespaced(self.client, self.resource.namespace.as_str());

        let result = if self.context.is_download {
            download_file(pods, self.resource, self.context).await
        } else {
            upload_file(self.runtime, pods, self.resource, self.context).await
        };

        Some(CommandResult::TransferFile(result))
    }
}

async fn download_file(
    pods: Api<Pod>,
    resource: ResourceRef,
    context: TransferContext,
) -> Result<TransferFileResult, TransferFileError> {
    let source = Path::new(&context.from);
    let (dir, file) = split_path(source)?;

    let attach_params = build_attach_params(&context.container, false);

    let mut attached = pods
        .exec(pod_name(&resource), ["tar", "cf", "-", "-C", dir, file], &attach_params)
        .await?;

    let mut stdout = attached.stdout().ok_or(TransferFileError::MissingStdout)?;
    let mut tar_data = Vec::new();
    stdout.read_to_end(&mut tar_data).await?;

    tar::Archive::new(tar_data.as_slice()).unpack(&context.to)?;

    Ok(TransferFileResult {
        is_download: true,
        pod: resource.name.unwrap_or_default(),
        container: context.container,
        file: file.to_owned(),
    })
}

async fn upload_file(
    runtime: Handle,
    pods: Api<Pod>,
    resource: ResourceRef,
    context: TransferContext,
) -> Result<TransferFileResult, TransferFileError> {
    let file_name = get_file_name(&context.from);
    let tar_buffer = runtime
        .spawn_blocking({
            let _source = context.from.clone();
            let _file_name = file_name.clone();
            move || build_tar_blocking(_source, _file_name)
        })
        .await??;

    let attach_params = build_attach_params(&context.container, true);
    let mut attached = pods
        .exec(pod_name(&resource), ["tar", "xf", "-", "-C", &context.to], &attach_params)
        .await?;

    let mut stdin = attached.stdin().ok_or(TransferFileError::MissingStdin)?;
    let mut stderr = attached.stderr().ok_or(TransferFileError::MissingStderr)?;

    let stderr_task = runtime.spawn(async move {
        let mut err_output = String::new();
        stderr.read_to_string(&mut err_output).await?;
        Ok::<String, std::io::Error>(err_output)
    });

    stdin.write_all(&tar_buffer).await?;
    stdin.shutdown().await?;

    drop(stdin);

    let stderr_output = stderr_task
        .await
        .map_err(|err| TransferFileError::RemoteProcessError(err.to_string()))?
        .map_err(TransferFileError::TarIoError)?;

    if !stderr_output.is_empty() {
        return Err(TransferFileError::RemoteProcessError(stderr_output));
    }

    attached
        .join()
        .await
        .map_err(|err| TransferFileError::RemoteProcessError(err.to_string()))?;

    Ok(TransferFileResult {
        is_download: false,
        pod: resource.name.unwrap_or_default(),
        container: context.container,
        file: file_name,
    })
}

fn build_attach_params(container: &str, stdin: bool) -> AttachParams {
    AttachParams {
        container: Some(container.to_string()),
        stdin,
        stdout: true,
        stderr: true,
        tty: false,
        ..Default::default()
    }
}

fn pod_name(resource: &ResourceRef) -> &str {
    resource.name.as_deref().unwrap_or_default()
}

fn build_tar_blocking(path: String, name: String) -> Result<Vec<u8>, TransferFileError> {
    let mut buffer = Vec::new();
    let mut file = std::fs::File::open(path)?;

    let mut builder = tar::Builder::new(&mut buffer);
    builder.append_file(name, &mut file)?;
    builder.finish()?;
    drop(builder);

    Ok(buffer)
}

fn split_path(path: &Path) -> Result<(&str, &str), TransferFileError> {
    let dir = path
        .parent()
        .and_then(|p| p.to_str())
        .ok_or_else(|| TransferFileError::InvalidPath(path.to_path_buf()))?;

    let file_name = path
        .file_name()
        .and_then(|f| f.to_str())
        .ok_or_else(|| TransferFileError::InvalidPath(path.to_path_buf()))?;

    Ok((dir, file_name))
}

fn get_file_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or(path)
        .to_string()
}
