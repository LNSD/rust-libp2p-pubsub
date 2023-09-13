use std::task::{Context, Poll};

use super::context_handles::{InCtx, OnEventCtx, PollCtx};
use super::service_trait::Service;

/// A event handler is a stateful object that handle input events and emit output events.
///
/// > **Note**: Any type implementing the [`EventHandler`] trait will implicitly implement the
/// > [`Service`] trait as well. The event handler when polled will iterate and consume all the
/// > input mailbox events and pass them to the [`EventHandler::on_event`] method.
///
/// The state can only be mutated by handling input events. The service can emit events by
/// enqueueing them into the output mailbox.
///
/// See [`on_event`](EventHandler::on_event) method for details.
///
pub trait EventHandler: 'static {
    type InEvent: 'static;
    type OutEvent: 'static;

    /// Handle an input event.
    ///
    /// To emit an event, enqueueing it into the output mailbox, use the
    /// [`emit`](super::context_handles::OutCtx::emit) method. To emit a batch of events, use the
    /// [`emit_batch`](super::context_handles::OutCtx::emit_batch) method.
    ///
    /// ```ignore
    /// use libp2p_pubsub_common::service::{EventHandler, OnEventCtx};
    ///
    /// impl EventHandler for MyService {
    ///     type InEvent = MyInEvent;
    ///     type OutEvent = MyOutEvent;
    ///
    ///     fn on_event<'a>(&mut self, scv_cx: &mut impl OnEventCtx<'a, Self::OutEvent>, ev: Self::InEvent) {
    ///         // ...
    ///         scv_cx.emit(event);                  // Emit an event.
    ///         scv_cx.emit_batch([event1, event2]); // Emit a batch of events.
    ///         // ...
    ///     }
    /// }
    ///```
    fn on_event<'a>(&mut self, svc_cx: &mut impl OnEventCtx<'a, Self::OutEvent>, ev: Self::InEvent);
}

impl<H> Service for H
where
    H: EventHandler,
{
    type InEvent = H::InEvent;
    type OutEvent = H::OutEvent;

    fn poll<'a>(
        &mut self,
        svc_cx: impl PollCtx<'a, Self::InEvent, Self::OutEvent>,
        _cx: &mut Context<'_>,
    ) -> Poll<Self::OutEvent> {
        let (mut inbox_cx, outbox_cx) = svc_cx.split();

        let mut on_event_cx = outbox_cx;
        while let Some(event) = inbox_cx.pop_next() {
            self.on_event(&mut on_event_cx, event);
        }

        Poll::Pending
    }
}
