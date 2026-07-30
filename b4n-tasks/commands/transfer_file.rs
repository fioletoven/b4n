use b4n_kube::{ResourceRef, files::TransferContext};
use k8s_openapi::api::core::v1::Pod;
use kube::{Api, Client, api::AttachParams};
use std::path::{Path, PathBuf};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::runtime::Handle;
use tokio::task::JoinHandle;

use crate::commands::CommandResult;

/// Possible file transfer errors.
#[derive(thiserror::Error, Debug)]
pub enum TransferFileError {
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

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

    #[error("failed to resolve home directory on remote container")]
    HomeDirectoryResolutionError,

    #[error("destination path already exists: {0}")]
    DestinationExists(String),
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
            download_file(self.runtime, pods, self.resource, self.context).await
        } else {
            upload_file(self.runtime, pods, self.resource, self.context).await
        };

        Some(CommandResult::TransferFile(result))
    }
}

async fn download_file(
    runtime: Handle,
    pods: Api<Pod>,
    resource: ResourceRef,
    context: TransferContext,
) -> Result<TransferFileResult, TransferFileError> {
    if !context.overwrite_files && tokio::fs::try_exists(&context.to).await? {
        return Err(TransferFileError::DestinationExists(context.to.clone()));
    }

    let remote_from = resolve_remote_tilde(&pods, &resource, &context.container, context.from).await?;
    let source = Path::new(&remote_from);
    let (dir, file) = split_path(source)?;

    let attach_params = build_attach_params(&context.container).stderr(true);
    let mut attached = pods
        .exec(pod_name(&resource), ["tar", "cf", "-", "-C", dir, file], &attach_params)
        .await?;

    let mut stdout = attached.stdout().ok_or(TransferFileError::MissingStdout)?;

    let stderr = attached.stderr().ok_or(TransferFileError::MissingStderr)?;
    let stderr_task = runtime.spawn(read_to_string(stderr));

    let mut tar_data = Vec::new();
    stdout.read_to_end(&mut tar_data).await?;

    check_stderr(stderr_task).await?;
    check_process_status(&mut attached).await?;

    attached
        .join()
        .await
        .map_err(|err| TransferFileError::RemoteProcessError(err.to_string()))?;

    runtime
        .spawn_blocking({
            let _destination = context.to.clone();
            move || tar::Archive::new(tar_data.as_slice()).unpack(&_destination)
        })
        .await??;

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
    let remote_to = resolve_remote_tilde(&pods, &resource, &context.container, context.to).await?;
    let file_name = get_file_name(&context.from);

    if !context.overwrite_files {
        let path = format!("{}/{}", remote_to.trim_end_matches('/'), file_name);
        if remote_path_exists(&pods, &resource, &context.container, &path).await? {
            return Err(TransferFileError::DestinationExists(path));
        }
    }

    let tar_buffer = runtime
        .spawn_blocking({
            let _source = context.from.clone();
            let _file_name = file_name.clone();
            move || build_tar_blocking(_source, _file_name)
        })
        .await??;

    let attach_params = build_attach_params(&context.container).stdin(true).stderr(true);
    let mut attached = pods
        .exec(pod_name(&resource), ["tar", "xf", "-", "-C", &remote_to], &attach_params)
        .await?;

    let mut stdin = attached.stdin().ok_or(TransferFileError::MissingStdin)?;

    let stderr = attached.stderr().ok_or(TransferFileError::MissingStderr)?;
    let stderr_task = runtime.spawn(read_to_string(stderr));

    stdin.write_all(&tar_buffer).await?;
    stdin.shutdown().await?;
    drop(stdin);

    check_stderr(stderr_task).await?;
    check_process_status(&mut attached).await?;

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

fn build_tar_blocking(path: String, name: String) -> Result<Vec<u8>, TransferFileError> {
    let mut buffer = Vec::new();
    let mut file = std::fs::File::open(path)?;

    let mut builder = tar::Builder::new(&mut buffer);
    builder.append_file(name, &mut file)?;
    builder.finish()?;
    drop(builder);

    Ok(buffer)
}

async fn resolve_remote_tilde(
    pods: &Api<Pod>,
    resource: &ResourceRef,
    container: &str,
    path: String,
) -> Result<String, TransferFileError> {
    if !path.starts_with('~') {
        return Ok(path);
    }

    let home = exec_capture_stdout(pods, resource, container, "echo $HOME").await?;
    if home.is_empty() {
        return Err(TransferFileError::HomeDirectoryResolutionError);
    }

    Ok(path.replacen('~', &home, 1))
}

async fn remote_path_exists(
    pods: &Api<Pod>,
    resource: &ResourceRef,
    container: &str,
    path: &str,
) -> Result<bool, TransferFileError> {
    let command = format!("test -e '{}' && echo 1 || echo 0", path.replace('\'', "'\\''"));
    let output = exec_capture_stdout(pods, resource, container, &command).await?;

    Ok(output == "1")
}

async fn exec_capture_stdout(
    pods: &Api<Pod>,
    resource: &ResourceRef,
    container: &str,
    command: &str,
) -> Result<String, TransferFileError> {
    let attach_params = build_attach_params(container);
    let mut attached = pods.exec(pod_name(resource), ["sh", "-c", command], &attach_params).await?;

    let mut stdout = attached.stdout().ok_or(TransferFileError::MissingStdout)?;

    let mut output = String::new();
    stdout.read_to_string(&mut output).await?;

    check_process_status(&mut attached).await?;

    attached
        .join()
        .await
        .map_err(|err| TransferFileError::RemoteProcessError(err.to_string()))?;

    Ok(output.trim().to_string())
}

async fn check_process_status(attached: &mut kube::api::AttachedProcess) -> Result<(), TransferFileError> {
    if let Some(status_future) = attached.take_status()
        && let Some(status) = status_future.await
        && status.status.as_deref() != Some("Success")
    {
        return Err(TransferFileError::RemoteProcessError(status.message.unwrap_or_default()));
    }

    Ok(())
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

async fn read_to_string(mut reader: impl AsyncRead + Unpin) -> Result<String, std::io::Error> {
    let mut output = String::new();
    reader.read_to_string(&mut output).await?;

    Ok(output)
}

async fn check_stderr(task: JoinHandle<Result<String, std::io::Error>>) -> Result<(), TransferFileError> {
    let output = task
        .await
        .map_err(|err| TransferFileError::RemoteProcessError(err.to_string()))?
        .map_err(TransferFileError::IoError)?;

    let trimmed = output.trim();
    if !trimmed.is_empty() {
        return Err(TransferFileError::RemoteProcessError(trimmed.to_string()));
    }

    Ok(())
}

fn build_attach_params(container: &str) -> AttachParams {
    AttachParams {
        container: Some(container.to_string()),
        stdin: false,
        stdout: true,
        stderr: false,
        tty: false,
        ..Default::default()
    }
}

fn pod_name(resource: &ResourceRef) -> &str {
    resource.name.as_deref().unwrap_or_default()
}

fn get_file_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or(path)
        .to_string()
}
