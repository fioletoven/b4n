/// Represents context for items filtering.
pub trait FilterContext {
    /// Resets context data for new filtering.
    fn restart(&mut self);
}

/// Contract for items that allow filtering.
pub trait Filterable<Fc: FilterContext> {
    /// Builds [`FilterContext`] object that can be used to filter an item.
    fn get_context(pattern: &str, settings: Option<&str>) -> Fc;

    /// Checks if an item match a filter using the provided context.
    fn is_matching(&self, context: &mut Fc) -> bool;
}

/// Basic context that implements [`FilterContext`].
pub struct BasicFilterContext {
    pub pattern: String,
}

impl FilterContext for BasicFilterContext {
    fn restart(&mut self) {
        // Empty implementation.
    }
}

impl From<String> for BasicFilterContext {
    fn from(value: String) -> Self {
        BasicFilterContext { pattern: value }
    }
}

impl From<&str> for BasicFilterContext {
    fn from(value: &str) -> Self {
        BasicFilterContext {
            pattern: value.to_owned(),
        }
    }
}

/// Keeps all data needed for lists with [`Filterable`] items.
pub struct FilterData<Fc: FilterContext> {
    pattern: Option<String>,
    settings: Option<String>,
    context: Option<Fc>,
}

impl<Fc: FilterContext> Default for FilterData<Fc> {
    fn default() -> Self {
        Self {
            pattern: None,
            settings: None,
            context: None,
        }
    }
}

impl<Fc: FilterContext> FilterData<Fc> {
    /// Returns `true` if [`FilterData<Fc>`] contains any pattern.
    pub fn has_pattern(&self) -> bool {
        self.pattern.is_some()
    }

    /// Returns `true` if [`FilterData<Fc>`] contains any settings.
    pub fn has_settings(&self) -> bool {
        self.settings.is_some()
    }

    /// Returns `true` if [`FilterData<Fc>`] contains any context.
    pub fn has_context(&self) -> bool {
        self.context.is_some()
    }

    /// Gets settings for [`Filterable`] item.
    pub fn settings(&self) -> Option<&str> {
        self.settings.as_deref()
    }

    /// Sets settings for [`Filterable`] item. Returns `true` if settings were updated.\
    /// **Note** that it clears filter context.
    pub fn set_settings(&mut self, settings: Option<impl Into<String>>) -> bool {
        let new_settings = settings.map(Into::into);
        if self.settings != new_settings {
            self.settings = new_settings;
            self.context = None;
            true
        } else {
            false
        }
    }

    /// Gets pattern for [`Filterable`] item.
    pub fn pattern(&self) -> Option<&str> {
        self.pattern.as_deref()
    }

    /// Sets pattern for [`Filterable`] item. Returns `true` if pattern was updated.\
    /// **Note** that it clears filter context.
    pub fn set_pattern(&mut self, pattern: Option<impl Into<String>>) -> bool {
        let new_pattern = pattern.map(Into::into);
        if self.pattern != new_pattern {
            self.pattern = new_pattern;
            self.context = None;
            true
        } else {
            false
        }
    }

    /// Gets mutable context for [`Filterable`] item.
    pub fn context_mut(&mut self) -> Option<&mut Fc> {
        self.context.as_mut()
    }

    /// Sets context for [`Filterable`] item.
    pub fn set_context(&mut self, context: Option<Fc>) {
        self.context = context;
    }
}
