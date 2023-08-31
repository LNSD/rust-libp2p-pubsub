use std::task::{Context, Poll};

use common::service::{Context as ServiceContext, Service};

/// Create a No-op task `Context` for testing.
///
/// This is a convenience wrapper function around [`futures_test::task::noop_context()`] to create
/// a [`Context`]s for testing purposes.
pub fn noop_context() -> Context<'static> {
    futures_test::task::noop_context()
}

/// Convenience function to create a new `Service` with default values for testing.
pub fn default_test_service<S: Service + Default>() -> ServiceContext<S> {
    ServiceContext::default()
}

/// Convenience function to inject various events into a `Service`.
pub fn inject_events<S>(
    service: &mut ServiceContext<S>,
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
pub fn poll<S>(service: &mut ServiceContext<S>, cx: &mut Context<'_>)
where
    S: Service,
{
    while let Poll::Ready(_) = service.poll(cx) {}
}

/// Poll a [`Service`] and collect all events from it.
///
/// This function polls the service until it returns `Poll::Pending`. Returns a `Vec` of all events
/// emitted by the service.
pub fn collect_events<S>(service: &mut ServiceContext<S>, cx: &mut Context<'_>) -> Vec<S::OutEvent>
where
    S: Service,
{
    let mut events = Vec::new();
    while let Poll::Ready(event) = service.poll(cx) {
        events.push(event);
    }
    events
}
