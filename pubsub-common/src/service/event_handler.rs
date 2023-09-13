use std::task::{Context, Poll};

use super::context_handles::{InCtx, OnEventCtx, PollCtx};
use super::service_trait::Service;

/// A event handler is a stateful object that handle input events and emit output events.
///
/// The service state can only be mutated by handling input events. The service can emit events
/// either by enqueueing them into the output mailbox.
///
/// See [`on_event`](#method.on_event) method for more details.
///
/// This trait is a simplified version of the [`Service`] trait. This trait is a particular case of
/// the [`Service`] trait where the service only handles input events and emits output events, but
/// does not require to be polled.
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

/// Service implementation that wraps an [`EventHandler`] and implements the [`Service`]
/// trait.
///
/// The trait, when polled, iterates over the input mailbox events and calls the
/// [`EventHandler::on_event`] method for each event.
///
/// > NOTE: Use [`wrap_handler`] to wrap a [`EventHandler`] and return a [`ServiceWrapper`] instance.o
///>
/// > ```ignore
/// > let handler = MyHandler::new();  // Implements `EventHandler` trait.
/// > let my_service: BufferedContext<ServiceWrapper<MyHandler>> = BufferedContext::new(wrap_handler(handler));
/// > ```
pub struct ServiceWrapper<H> {
    inner: H,
}

impl<H> Default for ServiceWrapper<H>
where
    H: Default,
{
    fn default() -> Self {
        Self {
            inner: H::default(),
        }
    }
}

impl<H> std::ops::Deref for ServiceWrapper<H> {
    type Target = H;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<H> Service for ServiceWrapper<H>
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
            self.inner.on_event(&mut on_event_cx, event);
        }

        Poll::Pending
    }
}

pub fn wrap_handler<H: EventHandler>(handler: H) -> ServiceWrapper<H> {
    ServiceWrapper { inner: handler }
}
