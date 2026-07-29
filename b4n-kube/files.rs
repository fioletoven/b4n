/// File transfer context.
#[derive(Debug, Clone, PartialEq)]
pub struct TransferContext {
    pub is_download: bool,
    pub from: String,
    pub to: String,
    pub container: String,
    pub overwrite_files: bool,
}

impl TransferContext {
    /// Creates new [`TransferContext`] instance to download a file.
    pub fn download(from: String, to: String, container: String, overwrite_files: bool) -> Self {
        Self {
            is_download: true,
            from,
            to,
            container,
            overwrite_files,
        }
    }

    /// Creates new [`TransferContext`] instance to upload a file.
    pub fn upload(from: String, to: String, container: String, overwrite_files: bool) -> Self {
        Self {
            is_download: false,
            from,
            to,
            container,
            overwrite_files,
        }
    }
}
