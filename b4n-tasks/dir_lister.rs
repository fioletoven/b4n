use std::path::{Path, PathBuf};
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
    current_path: Option<PathBuf>,
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
            task: None,
            tx,
            rx,
        }
    }

    /// Resets [`DirLister`].
    pub fn reset(&mut self) {
        self.current_path = None;
        if let Some(handle) = self.task.take() {
            handle.abort();
        }
    }

    /// Starts listing a directory in the background.
    pub fn list_dir(&mut self, path: PathBuf, include_parent: bool) -> bool {
        if self.current_path.as_ref().is_some_and(|p| p == &path) {
            return false;
        }

        if let Some(handle) = self.task.take() {
            handle.abort();
        }

        self.current_path = Some(path.clone());

        let tx = self.tx.clone();
        let mut include_parent = include_parent;

        let handle = self.runtime.spawn(async move {
            let _ = tx.send(DirListResult::Init).await;
            if let Err(e) = Self::list_loop(&path, tx.clone(), &mut include_parent).await {
                let _ = tx.send(DirListResult::Error(e.to_string())).await;
            } else {
                if include_parent {
                    Self::include_parent(&path, &tx).await;
                }

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

    async fn list_loop(path: &Path, tx: mpsc::Sender<DirListResult>, include_parent: &mut bool) -> Result<(), std::io::Error> {
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
