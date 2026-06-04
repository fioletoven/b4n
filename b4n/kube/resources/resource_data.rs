use b4n_config::themes::{ResourceColors, TextColors};
use b4n_kube::ResourceTag;

use crate::ui::widgets::table::Cell;

/// Extra data for the kubernetes resource.
#[derive(Default)]
pub struct ResourceData {
    pub is_completed: bool,
    pub is_ready: bool,
    pub is_terminating: bool,
    pub extra_values: Box<[Cell]>,
    pub tags: Box<[ResourceTag]>,
}

impl ResourceData {
    /// Creates new [`ResourceData`] instance.
    pub fn new(values: Box<[Cell]>, is_terminating: bool) -> Self {
        Self {
            extra_values: values,
            is_ready: !is_terminating,
            is_terminating,
            ..Default::default()
        }
    }

    /// Adds tags to the [`ResourceData`] object.
    pub fn with_tags(mut self, tags: Box<[ResourceTag]>) -> Self {
        self.tags = tags;
        self
    }

    /// Returns [`TextColors`] for the current resource state.
    pub fn get_colors(&self, colors: &ResourceColors, is_active: bool, is_selected: bool) -> TextColors {
        if self.is_completed {
            colors.completed.get_specific(is_active, is_selected)
        } else if self.is_terminating {
            colors.terminating.get_specific(is_active, is_selected)
        } else if self.is_ready {
            colors.ready.get_specific(is_active, is_selected)
        } else {
            colors.in_progress.get_specific(is_active, is_selected)
        }
    }
}
