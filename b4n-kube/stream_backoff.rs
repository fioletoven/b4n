use futures::{Stream, TryStream};
use kube::runtime::utils::Backoff;
use kube::runtime::watcher::{Error as WatcherError, Event};
use pin_project::pin_project;
use std::{marker::PhantomData, pin::Pin, task::Poll};
use tokio::time::{Instant, Sleep, sleep};

/// Based on https://github.com/kube-rs/kube/blob/2.0.1/kube-runtime/src/utils/stream_backoff.rs
/// If fixes the backoff reset on receiving Poll::Ready(Some(Ok(Event::Init))) which is sent also after errors.
///
/// Applies a [`Backoff`] policy to a [`Stream`]
///
/// After any [`Err`] is emitted, the stream is paused for [`Backoff::next`]. The
/// [`Backoff`] is [`reset`](`Backoff::reset`) on any [`Ok`] value.
///
/// If [`Backoff::next`] returns [`None`] then the backing stream is given up on, and closed.
#[pin_project]
pub struct StreamBackoff<S, B, K> {
    #[pin]
    stream: S,
    backoff: B,
    #[pin]
    state: State,
    _phantom: PhantomData<K>,
}

#[pin_project(project = StreamBackoffStateProj)]
// It's expected to have relatively few but long-lived `StreamBackoff`s in a project, so we would rather have
// cheaper sleeps than a smaller `StreamBackoff`.
#[allow(clippy::large_enum_variant)]
enum State {
    BackingOff(#[pin] Sleep),
    GivenUp,
    Awake,
}

impl<S, B, K> StreamBackoff<S, B, K> {
    pub fn new(stream: S, backoff: B) -> Self {
        Self {
            stream,
            backoff,
            state: State::Awake,
            _phantom: PhantomData,
        }
    }
}

impl<S, B, K> Stream for StreamBackoff<S, B, K>
where
    S: TryStream<Ok = Event<K>, Error = WatcherError>,
    B: Backoff,
{
    type Item = Result<Event<K>, WatcherError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        match this.state.as_mut().project() {
            StreamBackoffStateProj::BackingOff(mut backoff_sleep) => match backoff_sleep.as_mut().poll(cx) {
                Poll::Ready(()) => {
                    tracing::debug!(deadline = ?backoff_sleep.deadline(), "Backoff complete, waking up");
                    this.state.set(State::Awake)
                },
                Poll::Pending => {
                    let deadline = backoff_sleep.deadline();
                    tracing::trace!(
                        ?deadline,
                        remaining_duration = ?deadline.saturating_duration_since(Instant::now()),
                        "Still waiting for backoff sleep to complete"
                    );
                    return Poll::Pending;
                },
            },
            StreamBackoffStateProj::GivenUp => {
                tracing::debug!("Backoff has given up, stream is closed");
                return Poll::Ready(None);
            },
            StreamBackoffStateProj::Awake => {},
        }

        let next_item = this.stream.try_poll_next(cx);
        match &next_item {
            Poll::Ready(Some(Err(_))) => {
                if let Some(backoff_duration) = this.backoff.next() {
                    let backoff_sleep = sleep(backoff_duration);
                    tracing::debug!(
                        deadline = ?backoff_sleep.deadline(),
                        duration = ?backoff_duration,
                        "Error received, backing off"
                    );
                    this.state.set(State::BackingOff(backoff_sleep));
                } else {
                    tracing::debug!("Error received, giving up");
                    this.state.set(State::GivenUp);
                }
            },
            Poll::Ready(Some(Ok(Event::Init))) => {
                tracing::trace!("Intercepted Init event without resetting backoff");
            },
            Poll::Ready(_) => {
                // Reset only on non-Init success events (e.g., InitApply, InitDone, Apply, Delete)
                tracing::trace!("Non-error, non-Init received, resetting backoff");
                this.backoff.reset();
            },
            Poll::Pending => {},
        }
        next_item
    }
}
