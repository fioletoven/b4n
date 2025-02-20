use std::time::Instant;
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    app::{
        SharedAppData, SharedBgWorker,
        commands::{Command, KubernetesClientError, KubernetesClientResult, NewKubernetesClientCommand},
        utils::StateChangeTracker,
    },
    kubernetes::Namespace,
    ui::widgets::FooterMessage,
};

/// Kubernetes client request data.
struct RequestInfo {
    request_time: Instant,
    request_id: Option<String>,
    context: String,
    kind: String,
    namespace: Namespace,
}

impl RequestInfo {
    /// Returns `true` if request match the specified ID.
    pub fn request_match(&self, request_id: &str) -> bool {
        self.request_id.as_deref().is_some_and(|id| id == request_id)
    }

    /// Returns `true` if there is no request pending and last request was more than 30 seconds ago.
    pub fn is_overdue(&self) -> bool {
        self.request_id.is_none() && self.request_time.elapsed().as_secs() > 30
    }
}

/// Kubernetes client manager.
pub struct KubernetesClientManager {
    app_data: SharedAppData,
    worker: SharedBgWorker,
    request: Option<RequestInfo>,
    messages_tx: UnboundedSender<FooterMessage>,
    connection_state: StateChangeTracker<bool>,
}

impl KubernetesClientManager {
    /// Creates new [`KubernetesClientManager`] instance.
    pub fn new(app_data: SharedAppData, worker: SharedBgWorker, messages_tx: UnboundedSender<FooterMessage>) -> Self {
        Self {
            app_data,
            worker,
            request: None,
            messages_tx,
            connection_state: StateChangeTracker::new(false),
        }
    }

    /// Sends command to create new Kubernetes client to the background executor.
    pub fn request_new_client(&mut self, context: String, kind: String, namespace: Namespace) {
        if let Some(connecting) = &self.request {
            self.worker.borrow_mut().cancel_command(connecting.request_id.as_deref());
        }

        self.request = Some(self.new_kubernetes_client(context, kind, namespace));
    }

    /// Clears the current Kubernetes request data.  
    /// **Note** that the request can be canceled first with `cancel_first`.
    pub fn erase_request(&mut self, cancel_first: bool) {
        if cancel_first {
            if let Some(connecting) = &self.request {
                self.worker.borrow_mut().cancel_command(connecting.request_id.as_deref());
            }
        }

        self.request = None;
    }

    /// Sets the current Kubernetes request as faulty.  
    /// **Note** that it will not match any new command ID.
    pub fn set_request_as_faulty(&mut self) {
        if let Some(connecting) = &mut self.request {
            connecting.request_id = None;
        }
    }

    /// Checks if current Kubernetes request is overdue and creates new one if it is.
    pub fn process_request_overdue(&mut self) {
        if self.request.as_ref().is_some_and(|c| c.is_overdue()) {
            if let Some(connecting) = self.request.take() {
                self.worker.borrow_mut().cancel_command(connecting.request_id.as_deref());
                self.request = Some(self.new_kubernetes_client(connecting.context, connecting.kind, connecting.namespace));
            }
        }
    }

    /// Processes result from the Kubernetes client request.
    pub fn process_result(
        &mut self,
        command_id: &str,
        result: Result<KubernetesClientResult, KubernetesClientError>,
    ) -> Option<KubernetesClientResult> {
        if self.request_match(command_id) {
            match result {
                Ok(result) => Some(result),
                Err(err) => {
                    self.set_request_as_faulty();
                    let msg = format!("Requested client error: {}.", err);
                    self.messages_tx.send(FooterMessage::error(msg, 10_000)).unwrap();
                    None
                }
            }
        } else {
            None
        }
    }

    /// Checks if the request matches provided `command_id`.
    #[inline]
    pub fn request_match(&self, command_id: &str) -> bool {
        self.request.as_ref().is_some_and(|c| c.request_match(command_id))
    }

    /// Returns `true` if manager is currently waiting for a new Kubernetes client.
    #[inline]
    pub fn is_requested(&self) -> bool {
        self.request.is_some()
    }

    /// Returns `true` if connection state changed and disconnection event should be processed.
    pub fn should_process_disconnection(&mut self) -> bool {
        self.connection_state
            .changed_to(self.app_data.borrow().is_connected && !self.is_requested(), false)
    }

    /// Sends command to create new Kubernetes client to the background executor.
    fn new_kubernetes_client(&mut self, context: String, kind: String, namespace: Namespace) -> RequestInfo {
        let kube_config_path = self.app_data.borrow().config.kube_config_path().map(String::from);
        let cmd = NewKubernetesClientCommand::new(kube_config_path, context.clone(), kind.clone(), namespace.clone());

        RequestInfo {
            request_id: Some(
                self.worker
                    .borrow_mut()
                    .run_command(Command::NewKubernetesClient(Box::new(cmd))),
            ),
            request_time: Instant::now(),
            context,
            kind,
            namespace,
        }
    }
}
