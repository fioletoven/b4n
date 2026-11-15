/// Options for draining a node.\
/// **Note** that defaults preserve pod grace periods and data.
#[derive(Debug, Clone, PartialEq)]
pub struct DrainOptions {
    pub grace_period_seconds: Option<u32>,
    pub timeout_seconds: u32,
    pub ignore_daemonsets: bool,
    pub delete_empty_dir_data: bool,
    pub force: bool,
    pub disable_eviction: bool,
}

impl Default for DrainOptions {
    fn default() -> Self {
        Self {
            grace_period_seconds: None,
            timeout_seconds: 5,
            ignore_daemonsets: false,
            delete_empty_dir_data: false,
            force: false,
            disable_eviction: false,
        }
    }
}
