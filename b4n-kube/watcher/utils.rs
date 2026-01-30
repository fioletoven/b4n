use b4n_common::{DEFAULT_ERROR_DURATION, NotificationSink};
use kube::runtime::watcher::{self, Error};

/// Checks if specified watcher Error is an API error or any other error.\
/// `check_forbidden` - returns `true` only for forbidden (403) API error.
pub fn is_api_error(error: &watcher::Error, check_forbidden: bool) -> bool {
    match error {
        Error::InitialListFailed(kube::Error::Api(response))
        | Error::WatchStartFailed(kube::Error::Api(response))
        | Error::WatchError(response)
        | Error::WatchFailed(kube::Error::Api(response)) => !check_forbidden || response.is_forbidden(),
        _ => false,
    }
}

/// Logs error message to the logs file and to the notifications sing (typically footer).
pub fn log_error_message(msg: String, sink: Option<&NotificationSink>) {
    tracing::warn!("{}", msg);
    if let Some(sink) = sink {
        sink.show_error(msg, DEFAULT_ERROR_DURATION);
    }
}
