use std::path::PathBuf;
use tokio::fs;
use tokio::runtime::Handle;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

#[derive(Debug, Clone)]
pub struct DirEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
}

#[derive(Debug)]
pub enum DirListResult {
    Init,
    Entry(DirEntry),
    Complete,
    Error(String),
}

/// Allows to list directory in a background task.
pub struct DirLister {
    runtime: Handle,
    task: Option<JoinHandle<()>>,
    tx: mpsc::Sender<DirListResult>,
    rx: mpsc::Receiver<DirListResult>,
    include_parent: bool,
    current_path: Option<PathBuf>,
}

impl DirLister {
    /// Creates new [`DirLister`] instance.
    pub fn new(runtime: Handle, buffer_size: usize) -> Self {
        let (tx, rx) = mpsc::channel(buffer_size);
        Self {
            runtime,
            task: None,
            tx,
            rx,
            include_parent: false,
            current_path: None,
        }
    }

    /// Sets whether to include parent directory (..) in the listing.
    pub fn with_parent(mut self, include_parent: bool) -> Self {
        self.include_parent = include_parent;
        self
    }

    /// Sets whether to include parent directory (..) in the listing.
    pub fn set_include_parent(&mut self, include_parent: bool) {
        self.include_parent = include_parent;
    }

    /// Starts listing a directory in the background.
    pub fn list_dir(&mut self, path: PathBuf) -> bool {
        if self.current_path.as_ref().is_some_and(|p| p == &path) {
            return false;
        }

        if let Some(handle) = self.task.take() {
            handle.abort();
        }

        self.current_path = Some(path.clone());

        let tx = self.tx.clone();
        let include_parent = self.include_parent;

        let handle = self.runtime.spawn(async move {
            let _ = tx.send(DirListResult::Init).await;
            if let Err(e) = Self::list_directory(path, tx.clone(), include_parent).await {
                let _ = tx.send(DirListResult::Error(e.to_string())).await;
            } else {
                let _ = tx.send(DirListResult::Complete).await;
            }
        });

        self.task = Some(handle);
        true
    }

    /// Tries to receive the next result.
    pub fn try_recv(&mut self) -> Option<DirListResult> {
        self.rx.try_recv().ok()
    }

    async fn list_directory(path: PathBuf, tx: mpsc::Sender<DirListResult>, include_parent: bool) -> Result<(), std::io::Error> {
        if include_parent && let Some(parent) = path.parent() {
            let parent_entry = DirEntry {
                name: "..".to_string(),
                path: parent.to_path_buf(),
                is_dir: true,
            };
            if tx.send(DirListResult::Entry(parent_entry)).await.is_err() {
                return Ok(());
            }
        }

        let mut entries = fs::read_dir(&path).await?;

        while let Some(entry) = entries.next_entry().await? {
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
}

impl Drop for DirLister {
    fn drop(&mut self) {
        if let Some(handle) = self.task.take() {
            handle.abort();
        }
    }
}
