use b4n_kube::{PropagationPolicy, ResourceRef, ResourceRefFilter, ResourceTag, Scope};

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

/// Data for items to select (highlight) on the resource list.
#[derive(Debug, Clone, PartialEq)]
pub enum ToSelectData {
    Some(String, String),
    None,
}

impl ToSelectData {
    /// Creates new `Some` variant of `ToSelectData`.
    pub fn new(name: impl Into<String>, namespace: Option<impl Into<String>>) -> Self {
        Self::Some(name.into(), namespace.map(Into::into).unwrap_or_default())
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
    ChangeAndSelect(String, String, ToSelectData),
    ChangeAndSelectPrev(String, String, ToSelectData),
    ChangeKind(String),
    ChangeKindAndSelect(String, ToSelectData),
    ChangeNamespace(String),
    ChangeContext(String, Option<String>),
    ChangeTheme(String),

    ViewPreviousResource,
    ViewContainers(String, String),
    ViewInvolved(String, String, ToSelectData),
    ViewScoped(String, Option<String>, ToSelectData, ScopeData),
    ViewScopedPrev(String, Option<String>, ToSelectData, ScopeData),
    ViewNamespaces,
    ListNamespaces,

    ListKubeContexts,
    ListThemes,
    ListResourcePorts(ResourceRef),

    AskDeleteResources,
    DeleteResources(PropagationPolicy, bool, bool),

    NewYaml(ResourceRef, bool),
    ViewYaml(ResourceRef, bool, bool),
    ViewLogs(ResourceRef, Option<Vec<ResourceTag>>),
    ViewPreviousLogs(ResourceRef, Option<Vec<ResourceTag>>),
    Describe(ResourceRef),

    AttachContainer(ResourceRef),
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
