use kube::api::{ApiResource, DynamicObject};
use kube::discovery::{ApiCapabilities, Scope, verbs};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use uuid::Uuid;

use crate::crds::CrdColumns;
use crate::{CONTAINERS, ResourceRef};

pub type ObserverResultSender = UnboundedSender<Box<ObserverResult<DynamicObject>>>;
pub type ObserverResultReceiver = UnboundedReceiver<Box<ObserverResult<DynamicObject>>>;

/// Background observer result.
pub enum ObserverResult<T> {
    Init(Box<InitData>),
    InitDone,
    Apply(T),
    Delete(T),
}

impl<T> ObserverResult<T> {
    /// Creates new [`ObserverResult`] for resource.
    pub fn new(resource: T, is_delete: bool) -> Self {
        if is_delete {
            Self::Delete(resource)
        } else {
            Self::Apply(resource)
        }
    }
}

/// Data that is returned when [`BgObserver`] starts watching resource.
#[derive(Clone)]
pub struct InitData {
    pub uuid: String,
    pub resource: ResourceRef,
    pub kind: String,
    pub kind_plural: String,
    pub group: String,
    pub version: String,
    pub scope: Scope,
    pub crd: Option<CrdColumns>,
    pub has_metrics: bool,
    pub is_editable: bool,
    pub is_creatable: bool,
    pub is_deletable: bool,
}

impl Default for InitData {
    fn default() -> Self {
        Self {
            uuid: String::new(),
            resource: ResourceRef::default(),
            kind: String::new(),
            kind_plural: String::new(),
            group: String::new(),
            version: String::new(),
            scope: Scope::Cluster,
            crd: None,
            has_metrics: false,
            is_editable: false,
            is_creatable: false,
            is_deletable: false,
        }
    }
}

impl InitData {
    /// Creates new initial data for [`ObserverResult`].
    pub fn new(rt: &ResourceRef, ar: &ApiResource, cap: &ApiCapabilities, crd: Option<CrdColumns>, has_metrics: bool) -> Self {
        let kind = if rt.is_container() { "Container" } else { ar.kind.as_str() };
        let kind_plural = if rt.is_container() { CONTAINERS } else { ar.plural.as_str() };
        Self {
            uuid: Uuid::new_v4()
                .hyphenated()
                .encode_lower(&mut Uuid::encode_buffer())
                .to_owned(),
            resource: rt.clone(),
            kind: kind.to_owned(),
            kind_plural: kind_plural.to_lowercase(),
            group: ar.group.clone(),
            version: ar.version.clone(),
            scope: cap.scope.clone(),
            crd,
            has_metrics,
            is_editable: cap.supports_operation(verbs::PATCH),
            is_creatable: cap.supports_operation(verbs::CREATE),
            is_deletable: cap.supports_operation(verbs::DELETE),
        }
    }
}
