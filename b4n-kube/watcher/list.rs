use b4n_common::NotificationSink;
use kube::api::{DynamicObject, ListParams, ObjectList};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::time::Duration;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

use crate::utils::get_object_uid;
use crate::watcher::{client::ResourceClient, result::ObserverResultSender, utils};
use crate::{BgObserverState, InitData, Namespace, ObserverResult};

pub struct ListInput {
    pub init_data: InitData,
    pub context_tx: ObserverResultSender,
    pub footer_tx: Option<NotificationSink>,
    pub state: Arc<AtomicU8>,
    pub has_error: Arc<AtomicBool>,
    pub has_access: Arc<AtomicBool>,
}

pub async fn list(
    mut client: ResourceClient,
    input: ListInput,
    fields: Option<String>,
    labels: Option<String>,
    mut fallback_namespace: Option<Namespace>,
    stop_on_access_error: bool,
    cancellation_token: CancellationToken,
) {
    let mut params = ListParams::default();
    if let Some(filter) = fields.as_ref() {
        params = params.fields(filter);
    }
    if let Some(filter) = labels.as_ref() {
        params = params.labels(filter);
    }

    let mut results = None;
    while !cancellation_token.is_cancelled() {
        let resources = client.get_api().list(&params).await;
        match resources {
            Ok(objects) => {
                results = Some(emit_results(objects, results, &input.init_data, &input.context_tx));
                input.state.store(BgObserverState::Ready.into(), Ordering::Relaxed);
                input.has_error.store(false, Ordering::Relaxed);
                input.has_access.store(true, Ordering::Relaxed);
            },
            Err(error) => {
                results = None;
                let is_access_error = matches!(&error, kube::Error::Api(response) if response.is_forbidden());
                let is_connected = matches!(&error, kube::Error::Api(_)); // we can connect to API, but can't use it

                let state: BgObserverState = input.state.load(Ordering::Relaxed).into();
                if is_connected && (state != BgObserverState::Ready) {
                    input.state.store(BgObserverState::Connected.into(), Ordering::Relaxed);
                }

                input.has_error.store(true, Ordering::Relaxed);
                input.has_access.store(!is_access_error, Ordering::Relaxed);
                if is_access_error {
                    if let Some(ns) = fallback_namespace.take() {
                        client.set_namespace(ns);
                        continue;
                    } else if stop_on_access_error {
                        break;
                    }
                }

                utils::log_error_message(
                    format!("Cannot list resource {}: {:?}", input.init_data.kind_plural, error),
                    input.footer_tx.as_ref(),
                );
            },
        }

        tokio::select! {
            () = cancellation_token.cancelled() => (),
            () = sleep(Duration::from_millis(5_000)) => (),
        }
    }
}

fn emit_results(
    objects: ObjectList<DynamicObject>,
    prev_results: Option<HashMap<String, DynamicObject>>,
    init_data: &InitData,
    context_tx: &ObserverResultSender,
) -> HashMap<String, DynamicObject> {
    let result = objects.items.iter().map(|o| (get_object_uid(o), o.clone())).collect();
    if let Some(mut prev_results) = prev_results {
        for object in objects {
            prev_results.remove(&get_object_uid(&object));
            let _ = context_tx.send(Box::new(ObserverResult::new(object, false)));
        }

        for (_, object) in prev_results {
            let _ = context_tx.send(Box::new(ObserverResult::new(object, true)));
        }
    } else {
        let _ = context_tx.send(Box::new(ObserverResult::Init(Box::new(init_data.clone()))));
        for object in objects {
            let _ = context_tx.send(Box::new(ObserverResult::new(object, false)));
        }

        let _ = context_tx.send(Box::new(ObserverResult::InitDone));
    }

    result
}
