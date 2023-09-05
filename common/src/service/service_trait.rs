#![allow(unused_variables)]

use std::task::{Context, Poll};

use crate::service::context_handles::{OnEventCtx, PollCtx};

pub type InEvent<T> = <T as Service>::InEvent;
pub type OutEvent<T> = <T as Service>::OutEvent;

/// A service is a stateful object that handle input events and can be polled for events.
///
/// The service state can only be mutated by handling input events. The service can emit events
/// either by enqueueing them into the output mailbox or by returning a `Poll::Ready` event.
///
/// Typically a service implementation only needs to implement either the [`Service::on_event`]
/// method or the [`Service::poll`] method. The [`Service::on_event`] method is useful for services
/// that are event-driven, while the [`Service::poll`] method is useful for services that are
/// polling-driven.
///
/// See [`on_event`](#method.on_event) and [`poll`](#method.poll) methods for more details.
pub trait Service: 'static {
    type InEvent: 'static;
    type OutEvent: 'static;

    /// Handle an input event.
    ///
    /// To emit an event, enqueueing it into the output mailbox, use the [`OnEventCtx::emit`]
    /// method. To emit a batch of events, use the [`OnEventCtx::emit_batch`] method.
    ///
    /// ```ignore
    /// use common::service::Service;
    /// use common::service::OnEventCtx;
    ///
    /// impl Service for MyService {
    ///     type InEvent = MyInEvent;
    ///     type OutEvent = MyOutEvent;
    ///
    ///     fn on_event(&mut self, scv_cx: &mut OnEventCtx<'_, Self::OutEvent>, ev: Self::InEvent) {
    ///         // ...
    ///         scv_cx.emit(event);                  // Emit an event.
    ///         scv_cx.emit_batch([event1, event2]); // Emit a batch of events.
    ///         // ...
    ///     }
    /// }
    ///```
    fn on_event(&mut self, svc_cx: &mut OnEventCtx<'_, Self::OutEvent>, ev: Self::InEvent) {}

    /// Poll the service for events.
    ///
    /// There are different ways to access the service context input mailbox events:
    ///
    ///  - Using the [`PollCtx::len`] and [`PollCtx::is_empty`] methods to check if there are any
    ///    events in the input mailbox. This can be useful to apply backpressure to the upstream
    ///    service.
    ///  - Using the [`PollCtx::pop_next`] method to pop the next event from the input mailbox.
    ///  - Using the [`PollCtx::drain`] method to get a `Drain` iterator over the input mailbox events.
    ///
    /// On the other hand, to emit an event, one can use one of the following methods:
    ///  - Using the the [`PollCtx::emit`] to enqueue an event to the output mailbox. Or using the
    ///    [`PollCtx::emit_batch`] method to enqueue a batch of events.
    ///  - Returning a `Poll::Ready(event)` to skip the output mailbox and return the event
    ///    directly to the downstream service.
    ///
    /// ```ignore
    /// use common::service::Service;
    /// use common::service::PollCtx;
    ///
    /// impl Service for MyService {
    ///     type InEvent = MyInEvent;
    ///     type OutEvent = MyOutEvent;
    ///
    ///     fn poll(&mut self, scv_cx: &mut PollCtx<'_, Self::InEvent, Self::OutEvent>, cx: &mut Context<'_>) {
    ///         // ...
    ///         scv_cx.emit(event);                  // Emit an event.
    ///         scv_cx.emit_batch([event1, event2]); // Emit a batch of events.
    ///         // ...
    ///         Poll::Ready(event);                  // Return an event to the downstream service.
    ///     }
    /// }
    ///```
    fn poll(
        &mut self,
        svc_cx: PollCtx<'_, Self::InEvent, Self::OutEvent>,
        cx: &mut Context<'_>,
    ) -> Poll<Self::OutEvent> {
        let (mut inbox_cx, outbox_cx) = svc_cx.split();

        let mut on_event_cx = outbox_cx.into();
        while let Some(event) = inbox_cx.pop_next() {
            self.on_event(&mut on_event_cx, event);
        }

        Poll::Pending
    }
}
