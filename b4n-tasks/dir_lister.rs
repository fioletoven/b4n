use k8s_openapi::api::core::v1::Pod;
use kube::{Api, api::AttachParams};
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncRead, AsyncReadExt, BufReader};
use tokio::runtime::Handle;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

/// Represents a directory entry.
#[derive(Debug, Clone)]
pub struct DirEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
}

/// Represents the result of a directory listing operation.
#[derive(Debug)]
pub enum DirListResult {
    Init,
    Resolved(PathBuf),
    Entry(DirEntry),
    Complete,
    Error(String),
}

/// Represents the context for remote directory listing.
#[derive(Clone)]
pub struct RemoteContext {
    client: Api<Pod>,
    pod: String,
    container: Option<String>,
}

impl RemoteContext {
    /// Creates a new [`RemoteContext`] instance.
    pub fn new(client: Api<Pod>, pod: String, container: Option<String>) -> Self {
        Self { client, pod, container }
    }
}

/// Allows to list directory in a background task.
pub struct DirLister {
    runtime: Handle,
    current_path: Option<PathBuf>,
    remote: Option<RemoteContext>,
    task: Option<JoinHandle<()>>,
    tx: mpsc::Sender<DirListResult>,
    rx: mpsc::Receiver<DirListResult>,
}

impl DirLister {
    /// Creates new [`DirLister`] instance.
    pub fn new(runtime: Handle, buffer_size: usize) -> Self {
        let (tx, rx) = mpsc::channel(buffer_size);
        Self {
            runtime,
            current_path: None,
            remote: None,
            task: None,
            tx,
            rx,
        }
    }

    /// Sets the current path for the directory listing.
    pub fn set_current_path(&mut self, path: PathBuf) {
        self.current_path = Some(path);
    }

    /// Sets [`DirLister`] to list remote directories.
    pub fn set_remote(&mut self, remote: Option<RemoteContext>) {
        self.reset();
        self.remote = remote;
    }

    /// Returns `true` if [`DirLister`] lists remote directories.
    pub fn is_remote(&self) -> bool {
        self.remote.is_some()
    }

    /// Resets [`DirLister`].
    pub fn reset(&mut self) {
        self.current_path = None;
        if let Some(handle) = self.task.take() {
            handle.abort();
        }

        while self.rx.try_recv().is_ok() {}
    }

    /// Tries to receive the next result.
    pub fn try_recv(&mut self) -> Option<DirListResult> {
        self.rx.try_recv().ok()
    }

    /// Starts listing a directory in the background.
    pub fn list_dir(&mut self, path: PathBuf, include_parent: bool) -> bool {
        if self.current_path.as_deref() == Some(path.as_path()) {
            return false;
        }

        if let Some(handle) = self.task.take() {
            handle.abort();
        }

        self.current_path = Some(path.clone());
        let remote = self.remote.clone();

        let (tx, rx) = mpsc::channel(self.tx.max_capacity());
        self.tx = tx.clone();
        self.rx = rx;

        let handle = self.runtime.spawn(async move {
            let _ = tx.send(DirListResult::Init).await;

            let result = Self::run_list_dir(remote, &path, &tx, include_parent).await;
            let message = match result {
                Ok(()) => DirListResult::Complete,
                Err(error) => DirListResult::Error(error),
            };

            let _ = tx.send(message).await;
        });

        self.task = Some(handle);
        true
    }

    async fn run_list_dir(
        remote: Option<RemoteContext>,
        path: &Path,
        tx: &mpsc::Sender<DirListResult>,
        include_parent: bool,
    ) -> Result<(), String> {
        if let Some(remote) = remote {
            return Self::list_remote(remote, path, tx.clone(), include_parent).await;
        }

        let mut include_parent = include_parent;
        Self::list_local(path, tx.clone(), &mut include_parent)
            .await
            .map_err(|error| error.to_string())?;

        if include_parent {
            Self::include_parent(path, tx).await;
        }

        Ok(())
    }

    async fn list_remote(
        context: RemoteContext,
        path: &Path,
        tx: mpsc::Sender<DirListResult>,
        include_parent: bool,
    ) -> Result<(), String> {
        let path = path.to_string_lossy();
        let params = AttachParams {
            container: context.container,
            ..Default::default()
        };

        let mut attached = context
            .client
            .exec(&context.pod, ["sh", "-c", REMOTE_LIST_SCRIPT, "b4n-list", &path], &params)
            .await
            .map_err(|error| error.to_string())?;

        let stdout = attached.stdout().ok_or("remote listing has no stdout")?;
        let mut stderr = attached.stderr().ok_or("remote listing has no stderr")?;

        let read_entries_task = list_remote_entries(stdout, tx, include_parent);
        let read_stderr_task = async {
            let mut output = String::new();
            stderr.read_to_string(&mut output).await.map_err(|error| error.to_string())?;
            Ok::<_, String>(output)
        };

        let (entries_result, stderr_result) = tokio::join!(read_entries_task, read_stderr_task);
        let stderr_output = stderr_result?;
        let status = attached.take_status().ok_or("remote listing failed")?.await;

        if status.and_then(|status| status.status).as_deref() != Some("Success") {
            return Err(if stderr_output.is_empty() {
                "remote listing failed".into()
            } else {
                stderr_output
            });
        }

        attached.join().await.map_err(|error| error.to_string())?;
        entries_result
    }

    async fn list_local(path: &Path, tx: mpsc::Sender<DirListResult>, include_parent: &mut bool) -> Result<(), std::io::Error> {
        if let Ok(path) = fs::canonicalize(path).await {
            let _ = tx.send(DirListResult::Resolved(path)).await;
        }

        let mut entries = fs::read_dir(&path).await?;
        while let Some(entry) = entries.next_entry().await? {
            if *include_parent {
                Self::include_parent(path, &tx).await;
                *include_parent = false;
            }

            let metadata = entry.metadata().await?;
            let name = entry.file_name().to_string_lossy().to_string();
            let path = entry.path();
            let is_dir = metadata.is_dir();

            let dir_entry = DirEntry { name, path, is_dir };
            if tx.send(DirListResult::Entry(dir_entry)).await.is_err() {
                break;
            }
        }

        Ok(())
    }

    async fn include_parent(path: &Path, tx: &mpsc::Sender<DirListResult>) {
        if let Some(parent) = path.parent() {
            let parent_entry = DirEntry {
                name: "..".to_string(),
                path: parent.to_path_buf(),
                is_dir: true,
            };
            let _ = tx.send(DirListResult::Entry(parent_entry)).await;
        }
    }
}

impl Drop for DirLister {
    fn drop(&mut self) {
        if let Some(handle) = self.task.take() {
            handle.abort();
        }
    }
}

/// Parses the output of `REMOTE_LIST_SCRIPT` and sends results through the provided channel.
///
/// The output starts with the current directory followed by zero or more kind/name pairs.
/// Each field is NUL-terminated: `current_directory\0kind\0entry_name\0...`.
async fn list_remote_entries(
    stdout: impl AsyncRead + Unpin,
    tx: mpsc::Sender<DirListResult>,
    include_parent: bool,
) -> Result<(), String> {
    let mut reader = BufReader::new(stdout);

    let current_dir = read_fragment(&mut reader).await?.ok_or("incomplete remote record")?;
    if !current_dir.starts_with('/') {
        return Err("remote directory is not an absolute POSIX path".into());
    }

    let _ = tx.send(DirListResult::Resolved(PathBuf::from(&current_dir))).await;
    if include_parent && current_dir != "/" {
        let _ = tx
            .send(DirListResult::Entry(DirEntry {
                name: "..".into(),
                path: PathBuf::from(format!("{current_dir}/..")),
                is_dir: true,
            }))
            .await;
    }

    while let Some(kind) = read_fragment(&mut reader).await? {
        let is_dir = match kind.as_str() {
            "d" => true,
            "f" => false,
            _ => return Err("invalid remote entry type".into()),
        };

        let name = read_fragment(&mut reader).await?.ok_or("incomplete remote record")?;
        if name.is_empty() || name.contains('/') {
            return Err("invalid remote entry name".into());
        }

        let path = PathBuf::from(format!("{}/{name}", current_dir.trim_end_matches('/')));
        if tx.send(DirListResult::Entry(DirEntry { name, path, is_dir })).await.is_err() {
            break;
        }
    }

    Ok(())
}

const REMOTE_LIST_SCRIPT: &str = r#"
case "$1" in
    '~') set -- "$HOME" ;;
    '~/'*) set -- "$HOME/${1#\~/}" ;;
esac
case "$1" in
    /*) ;;
    *) set -- "./$1" ;;
esac
cd "$1" || exit 1
if [ ! -r . ] || [ ! -x . ]; then
    printf '%s\n' 'directory is not readable' >&2
    exit 1
fi
printf '%s\000' "$PWD"
for entry in .[!.]* ..?* *; do
    if [ -L "$entry" ]; then
        if [ -d "$entry" ]; then kind=d; else kind=f; fi
    elif [ -d "$entry" ] && [ -r "$entry" ] && [ -x "$entry" ]; then
        kind=d
    elif [ -f "$entry" ] && [ -r "$entry" ]; then
        kind=f
    else
        continue
    fi
    printf '%s\000%s\000' "$kind" "$entry"
done
"#;

/// Reads a null-terminated fragment from the remote directory listing.\
/// Returns `Ok(None)` if the end of the stream is reached.
async fn read_fragment(reader: &mut (impl AsyncBufRead + Unpin)) -> Result<Option<String>, String> {
    let mut record = Vec::new();
    if reader.read_until(0, &mut record).await.map_err(|error| error.to_string())? == 0 {
        return Ok(None);
    }

    if record.pop() != Some(0) {
        return Err("incomplete remote record".into());
    }

    String::from_utf8(record)
        .map(Some)
        .map_err(|_| "remote record is not valid UTF-8".into())
}
