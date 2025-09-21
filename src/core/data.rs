use arboard::Clipboard;
use kube::discovery::Scope;
use std::{cell::RefCell, collections::HashSet, rc::Rc};
use syntect::{dumps::from_uncompressed_data, parsing::SyntaxSet};

use crate::{
    kubernetes::{Kind, Namespace, ResourceRef, kinds::KindItem, resources::CONTAINERS, watchers::InitData},
    ui::{KeyBindings, KeyCombination, KeyCommand, TuiEvent, theme::Theme},
};

use super::{Config, History};

pub type SharedAppData = Rc<RefCell<AppData>>;

pub const SYNTAX_SET_DATA: &[u8] = include_bytes!("../../assets/syntaxes/syntaxes.packdump");

/// Kubernetes resources data.
pub struct ResourcesInfo {
    pub context: String,
    pub version: String,
    pub scope: Scope,
    pub resource: ResourceRef,
    pub namespace: Namespace,
    is_all_namespace: bool,
}

impl Default for ResourcesInfo {
    fn default() -> Self {
        Self {
            context: String::default(),
            version: String::default(),
            scope: Scope::Cluster,
            resource: ResourceRef::default(),
            namespace: Namespace::default(),
            is_all_namespace: false,
        }
    }
}

impl ResourcesInfo {
    /// Creates new [`ResourcesInfo`] instance from provided values.
    pub fn from(context: String, namespace: Namespace, version: String, scope: Scope) -> Self {
        Self {
            context,
            is_all_namespace: namespace.is_all(),
            namespace,
            version,
            scope,
            ..Default::default()
        }
    }

    /// Updates [`ResourcesInfo`] with data from the [`InitData`].\
    /// **Note** that this update do not change the flag `is_all_namespace`.
    /// This results in remembering if the `all` namespace was set by user or by [`InitData`].
    pub fn update_from(&mut self, data: &InitData) {
        self.resource = data.resource.clone();
        self.scope = data.scope.clone();

        // change the namespace only if resource is namespaced
        if self.scope == Scope::Namespaced {
            self.namespace = data.resource.namespace.clone();
        }
    }

    /// Returns `true` if specified `namespace` is equal to the currently held by [`ResourcesInfo`].\
    /// **Note** that it takes into account the flag for `all` namespace.
    pub fn is_all_namespace(&self) -> bool {
        if self.is_all_namespace {
            true
        } else {
            self.namespace.is_all()
        }
    }

    /// Returns `true` if specified `namespace` is equal to the currently held by [`ResourcesInfo`].\
    /// **Note** that it takes into account the flag for `all` namespace.
    pub fn is_namespace_equal(&self, namespace: &Namespace) -> bool {
        if self.is_all_namespace {
            namespace.is_all()
        } else {
            self.namespace == *namespace
        }
    }

    /// Returns `true` if specified `kind` is equal to the currently held by [`ResourcesInfo`].
    pub fn is_kind_equal(&self, kind: &Kind) -> bool {
        (self.resource.is_container() && kind.as_str() == CONTAINERS)
            || (!self.resource.is_container() && &self.resource.kind == kind)
    }

    /// Sets new namespace.\
    /// **Note** that it takes into account the flag for `all` namespace.
    pub fn set_namespace(&mut self, namespace: Namespace) {
        self.is_all_namespace = namespace.is_all();
        self.namespace = namespace;
    }

    /// Gets namespace respecting the flag if it is an `all` namespace.
    pub fn get_namespace(&self) -> Namespace {
        if self.is_all_namespace {
            Namespace::all()
        } else {
            self.namespace.clone()
        }
    }
}

/// Keeps data required for syntax highlighting.
pub struct SyntaxData {
    pub syntax_set: SyntaxSet,
    pub yaml_theme: syntect::highlighting::Theme,
}

/// Contains all data that can be shared in the application.
#[derive(Default)]
pub struct AppData {
    /// Application configuration.
    pub config: Config,

    /// UI key bindings.
    pub key_bindings: KeyBindings,
    disabled_commands: HashSet<KeyCommand>,

    /// Application history data.
    pub history: History,

    /// Current application theme.
    pub theme: Theme,

    /// Information about currently selected kubernetes resource.
    pub current: ResourcesInfo,
    pub previous: Option<ResourceRef>,

    /// Holds all discovered kinds.
    pub kinds: Option<Vec<KindItem>>,

    /// Syntax set for syntax highlighting.
    pub syntax_set: SyntaxSet,

    /// Holds clipboard object.
    pub clipboard: Option<Clipboard>,

    /// Indicates if application is connected to the kubernetes api.
    pub is_connected: bool,
}

impl AppData {
    /// Creates new [`AppData`] instance.
    pub fn new(config: Config, history: History, theme: Theme) -> Self {
        let key_bindings = KeyBindings::default_with(config.key_bindings.clone());
        Self {
            config,
            key_bindings,
            disabled_commands: HashSet::default(),
            history,
            theme,
            current: ResourcesInfo::default(),
            previous: None,
            kinds: None,
            syntax_set: from_uncompressed_data::<SyntaxSet>(SYNTAX_SET_DATA).expect("cannot load SyntaxSet"),
            clipboard: Clipboard::new().ok(),
            is_connected: false,
        }
    }

    /// Returns resource's `kind` and `namespace` from the history data.\
    /// **Note** that if provided `context` is not found in the history file, current context resource is used.
    pub fn get_namespaced_resource_from_config(&self, context: &str) -> (Kind, Namespace) {
        if let Some(kind) = self.history.get_kind(context) {
            let namespace = self.history.get_namespace(context).unwrap_or_default();
            (kind.into(), namespace.into())
        } else {
            (self.current.resource.kind.clone(), self.current.namespace.clone())
        }
    }

    /// Returns new [`SyntaxData`] instance.\
    /// **Note** that all elements are cloned/build every time you call this method.
    pub fn get_syntax_data(&self) -> SyntaxData {
        SyntaxData {
            syntax_set: self.syntax_set.clone(),
            yaml_theme: self.theme.build_syntect_yaml_theme(),
        }
    }

    /// Sets the current resource as a previous one.
    pub fn update_previous(&mut self) {
        self.previous = Some(self.current.resource.clone());
    }

    /// Resets previous resource to `None`.
    pub fn reset_previous(&mut self) {
        self.previous = None;
    }
}

/// Extension methods for the [`SharedAppData`] type.
pub trait SharedAppDataExt {
    /// Returns `true` if the given [`TuiEvent`] is a key event and is bound to the specified [`KeyCommand`] within
    /// the [`KeyBindings`] stored in [`SharedAppData`].
    fn has_binding(&self, event: &TuiEvent, command: KeyCommand) -> bool;

    /// Temporarily disables or enables the given [`KeyCommand`] from being matched by `has_binding`.
    fn disable_command(&self, command: KeyCommand, disable: bool);

    /// Returns the [`TuiEvent::Key`] associated with the specified [`KeyCommand`] from the [`KeyBindings`].
    fn get_event(&self, command: KeyCommand) -> TuiEvent;

    /// Returns the [`KeyCombination`] associated with the specified [`KeyCommand`] from the [`KeyBindings`].
    fn get_key(&self, command: KeyCommand) -> KeyCombination;
}

impl SharedAppDataExt for SharedAppData {
    fn has_binding(&self, event: &TuiEvent, command: KeyCommand) -> bool {
        if let TuiEvent::Key(key) = event {
            let data = self.borrow();
            !data.disabled_commands.contains(&command) && data.key_bindings.has_binding(key, command)
        } else {
            false
        }
    }

    fn disable_command(&self, command: KeyCommand, hide: bool) {
        if hide {
            self.borrow_mut().disabled_commands.insert(command);
        } else {
            self.borrow_mut().disabled_commands.remove(&command);
        }
    }

    fn get_event(&self, command: KeyCommand) -> TuiEvent {
        TuiEvent::Key(self.borrow().key_bindings.get_key(command).unwrap_or_default())
    }

    fn get_key(&self, command: KeyCommand) -> KeyCombination {
        self.borrow().key_bindings.get_key(command).unwrap_or_default()
    }
}
