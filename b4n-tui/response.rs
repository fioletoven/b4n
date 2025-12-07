use b4n_kube::{ResourceRef, ResourceRefFilter, Scope};

use crate::TuiEvent;

/// UI object that is responsive and can process TUI key/mouse events.
pub trait Responsive {
    /// Process UI key or mouse event.
    fn process_event(&mut self, event: &TuiEvent) -> ResponseEvent;
}

/// Data for [`ResponseEvent::ViewScoped`] event.
#[derive(Debug, Clone, PartialEq)]
pub struct ScopeData {
    pub header: Scope,
    pub list: Scope,
    pub filter: ResourceRefFilter,
}

impl ScopeData {
    /// Creates new [`ScopeData`] instance that shows namespace column.
    pub fn namespace_visible(filter: ResourceRefFilter) -> Self {
        Self {
            header: Scope::Namespaced,
            list: Scope::Namespaced,
            filter,
        }
    }

    /// Creates new [`ScopeData`] instance that hides namespace column.
    pub fn namespace_hidden(filter: ResourceRefFilter) -> Self {
        Self {
            header: Scope::Namespaced,
            list: Scope::Cluster,
            filter,
        }
    }
}

/// Terminal UI Response Event.
#[derive(Debug, Clone, Default, PartialEq)]
pub enum ResponseEvent {
    #[default]
    NotHandled,
    Handled,
    Cancelled,
    Accepted,
    Action(&'static str),

    ExitApplication,

    Change(String, String),
    ChangeAndSelect(String, String, Option<String>),
    ChangeAndSelectPrev(String, String, Option<String>),
    ChangeKind(String),
    ChangeKindAndSelect(String, Option<String>),
    ChangeNamespace(String),
    ChangeContext(String),
    ChangeTheme(String),

    ViewPreviousResource,
    ViewContainers(String, String),
    ViewInvolved(String, String, Option<String>),
    ViewScoped(String, Option<String>, Option<String>, ScopeData),
    ViewScopedPrev(String, Option<String>, Option<String>, ScopeData),
    ViewNamespaces,

    ListKubeContexts,
    ListThemes,
    ListResourcePorts(ResourceRef),

    AskDeleteResources,
    DeleteResources(bool, bool),

    NewYaml(ResourceRef, bool),
    ViewYaml(ResourceRef, bool),
    ViewLogs(ResourceRef),
    ViewPreviousLogs(ResourceRef),

    OpenShell(ResourceRef),
    ShowPortForwards,
    PortForward(ResourceRef, u16, u16, String),
}

impl ResponseEvent {
    /// Returns `true` if [`ResponseEvent`] is an action matching the provided name.
    pub fn is_action(&self, name: &str) -> bool {
        if let ResponseEvent::Action(action) = self {
            *action == name
        } else {
            false
        }
    }

    /// Conditionally transforms a [`ResponseEvent`] into a new [`ResponseEvent`], consuming the original.\
    /// **Note** that the transformation is performed by the `f` closure, which is executed **only** if the event
    /// is an action matching the specified `name`.
    pub fn when_action_then<F>(self, name: &str, f: F) -> Self
    where
        F: FnOnce() -> Self,
    {
        if self.is_action(name) { f() } else { self }
    }

    /// Conditionally transforms a [`ResponseEvent`] into a new [`ResponseEvent`], consuming the original.\
    /// **Note** that the transformation is performed by the `f` closure, which is executed **only** if the event
    /// matches the specified `other` event.
    pub fn when_event_then<F>(self, other: &ResponseEvent, f: F) -> Self
    where
        F: FnOnce() -> Self,
    {
        if &self == other { f() } else { self }
    }
}
