use std::task::{Context, Poll};

use super::context_handles::PollCtx;

pub type InEvent<T> = <T as Service>::InEvent;
pub type OutEvent<T> = <T as Service>::OutEvent;

/// A service is a stateful object that handle input events and can be polled for events.
///
/// The service state can only be mutated by handling input events. The service can emit events
/// either by enqueueing them into the output mailbox or by returning a `Poll::Ready` event.
///
/// See [`poll`](#method.poll) method for more details.
pub trait Service: 'static {
    /// The type of the service input events.
    type InEvent: 'static;
    /// The type of the service output events.
    type OutEvent: 'static;

    /// Poll the service for events.
    ///
    /// There are different ways to access the service context input mailbox events:
    ///
    ///  - Using the [`len`](super::context_handles::InCtx::len) and
    ///    [`is_empty`](super::context_handles::InCtx::is_empty) methods to check if there are any
    ///    events in the input mailbox. This can be useful to apply backpressure to the upstream
    ///    service.
    ///  - Using the [`pop_next`](super::context_handles::InCtx::pop_next) method to pop the next
    ///    event from the input mailbox.
    ///
    /// On the other hand, to emit an event, one can use one of the following methods:
    ///
    ///  - Using the the [`emit`](super::context_handles::OutCtx::emit) to enqueue an event to the
    ///    output mailbox. Or using the [`emit_batch`](super::context_handles::OutCtx::emit_batch)
    ///    method to enqueue a batch of events.
    ///  - Returning a `Poll::Ready(event)` to skip the output mailbox and return the event
    ///    directly to the downstream service.
    ///
    /// ```ignore
    /// use libp2p_pubsub_common::service::{Service, PollCtx};
    ///
    /// impl Service for MyService {
    ///     type InEvent = MyInEvent;
    ///     type OutEvent = MyOutEvent;
    ///
    ///     fn poll<'a>(&mut self, scv_cx: impl PollCtx<'a, Self::InEvent, Self::OutEvent>, cx: &mut Context<'_>) {
    ///         // ...
    ///         scv_cx.emit(event);                  // Emit an event.
    ///         scv_cx.emit_batch([event1, event2]); // Emit a batch of events.
    ///         // ...
    ///         Poll::Ready(event);                  // Return an event to the downstream service.
    ///     }
    /// }
    ///```
    fn poll<'a>(
        &mut self,
        svc_cx: impl PollCtx<'a, Self::InEvent, Self::OutEvent>,
        cx: &mut Context<'_>,
    ) -> Poll<Self::OutEvent>;
}
