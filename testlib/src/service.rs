use std::future::poll_fn;
use std::task::{Context, Poll};

use libp2p_pubsub_common::service::{BufferedContext, Service, ServiceContext};

/// Create a No-op task `Context` for testing.
///
/// This is a convenience wrapper function around [`futures_test::task::noop_context()`] to create
/// a [`Context`]s for testing purposes.
pub fn noop_context() -> Context<'static> {
    futures_test::task::noop_context()
}

/// Convenience function to create a new `Service` with default values wrapped in a
/// [`BufferedContext`] for testing purposes.
pub fn default_test_service<S: Service + Default>() -> BufferedContext<S> {
    BufferedContext::default()
}

/// Convenience function to inject various events into a `Service`.
pub fn inject_events<S>(
    service: &mut BufferedContext<S>,
    events: impl IntoIterator<Item = S::InEvent>,
) where
    S: Service,
{
    for event in events {
        service.do_send(event);
    }
}

/// Poll a `Service` until it returns `Poll::Pending`.
///
/// All events emitted by the service are discarded. See [`collect_events`] to collect all events
/// emitted by the service.
pub fn poll<S>(service: &mut BufferedContext<S>, cx: &mut Context<'_>)
where
    S: Service,
{
    while service.poll(cx).is_ready() {}
}

/// Poll a [`Service`] and collect all events from it.
///
/// This function polls the service until it returns `Poll::Pending`. Returns a `Vec` of all events
/// emitted by the service.
pub fn collect_events<S>(service: &mut BufferedContext<S>, cx: &mut Context<'_>) -> Vec<S::OutEvent>
where
    S: Service,
{
    let mut events = Vec::new();
    while let Poll::Ready(event) = service.poll(cx) {
        events.push(event);
    }
    events
}

/// Poll a [`Service`] during a given duration and collect all events from it.
///
/// All events emitted by the service are discarded. See [`async_collect_events`] to collect all events
/// emitted by the service.
pub async fn async_poll<S>(service: &mut BufferedContext<S>)
where
    S: Service,
{
    poll_fn(|cx| {
        while service.poll(cx).is_ready() {}
        Poll::Ready(())
    })
    .await
}

/// Poll asynchronously a [`Service`] and collect all events from it.
///
/// This function polls the service until it returns `Poll::Pending`. Returns a `Vec` of all events
/// emitted by the service.
pub async fn async_collect_events<S>(service: &mut BufferedContext<S>) -> Vec<S::OutEvent>
where
    S: Service,
{
    poll_fn(|cx| {
        let mut events = Vec::new();
        while let Poll::Ready(event) = service.poll(cx) {
            events.push(event);
        }
        Poll::Ready(events)
    })
    .await
}
