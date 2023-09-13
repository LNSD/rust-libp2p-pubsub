use std::collections::VecDeque;
use std::task::{Context, Poll};

use crate::service::context_handles::{InCtx, JointCtx, OutCtx};

use super::context::ServiceContext;
use super::service_trait::Service;

impl<'a, InEvent> InCtx<'a> for &'a mut VecDeque<InEvent> {
    type Event = InEvent;

    fn len(&self) -> usize {
        VecDeque::len(self)
    }

    fn is_empty(&self) -> bool {
        VecDeque::is_empty(self)
    }

    fn pop_next(&mut self) -> Option<Self::Event> {
        VecDeque::pop_front(self)
    }
}

impl<'a, OutEvent> OutCtx<'a> for &'a mut VecDeque<OutEvent> {
    type Event = OutEvent;

    fn emit(&mut self, ev: Self::Event) {
        VecDeque::push_back(self, ev);
    }

    fn emit_batch(&mut self, evs: impl IntoIterator<Item = Self::Event>) {
        VecDeque::extend(self, evs);
    }
}

/// A service context mailbox handle implementation for the
/// [`Service::on_event`](super::service_trait""Service::on_event) method.
pub struct BufferedOnEventCtx<'a, OutEvent> {
    outbox: &'a mut VecDeque<OutEvent>,
}

impl<'a, OutEvent> OutCtx<'a> for BufferedOnEventCtx<'a, OutEvent> {
    type Event = OutEvent;

    fn emit(&mut self, ev: Self::Event) {
        self.outbox.push_back(ev);
    }

    fn emit_batch(&mut self, evs: impl IntoIterator<Item = Self::Event>) {
        self.outbox.extend(evs);
    }
}

/// A service context mailbox handle implementation for the
/// [`Service::poll`](super::service_trait::Service::poll) method.
pub struct BufferedPollCtx<'a, InEvent, OutEvent> {
    inbox: &'a mut VecDeque<InEvent>,
    outbox: &'a mut VecDeque<OutEvent>,
}

impl<'a, InEvent, OutEvent> BufferedPollCtx<'a, InEvent, OutEvent> {
    /// Create a new service context mailbox handle.
    pub(super) fn new(
        inbox: &'a mut VecDeque<InEvent>,
        outbox: &'a mut VecDeque<OutEvent>,
    ) -> Self {
        Self { inbox, outbox }
    }
}

impl<'a, InEvent, OutEvent> JointCtx<'a, InEvent, OutEvent>
    for BufferedPollCtx<'a, InEvent, OutEvent>
{
    type InHandle = &'a mut VecDeque<InEvent>;
    type OutHandle = &'a mut VecDeque<OutEvent>;

    fn split(self) -> (Self::InHandle, Self::OutHandle) {
        (self.inbox, self.outbox)
    }
}

impl<'a, InEvent, OutEvent> InCtx<'a> for BufferedPollCtx<'a, InEvent, OutEvent> {
    type Event = InEvent;

    fn len(&self) -> usize {
        self.inbox.len()
    }

    fn is_empty(&self) -> bool {
        self.inbox.is_empty()
    }

    fn pop_next(&mut self) -> Option<Self::Event> {
        self.inbox.pop_front()
    }
}

impl<'a, InEvent, OutEvent> OutCtx<'a> for BufferedPollCtx<'a, InEvent, OutEvent> {
    type Event = OutEvent;

    fn emit(&mut self, ev: Self::Event) {
        self.outbox.push_back(ev);
    }

    fn emit_batch(&mut self, evs: impl IntoIterator<Item = Self::Event>) {
        self.outbox.extend(evs);
    }
}

/// A buffered service context.
///
/// This [`ServiceContext`] implementation is a wrapper around a service that provides a mailbox
/// for input events and a mailbox for output events. The mailboxes are implemented as FIFO queues
/// based on [`VecDeque`].
///
/// The input mailbox acts as a buffer for the events that are sent to the service. The events are
/// enqueued into the mailbox by calling the [`do_send`](#method.do_send) method. The mailbox
/// events are processed by the service on the next [`BufferedContext::poll`] call (see
/// [`poll`](#method.poll) for more details).
///
/// The service context implements `Deref` trait to the inner service, so it can be used as the
/// wrapped service itself.
pub struct BufferedContext<S: Service> {
    service: S,
    inbox: VecDeque<S::InEvent>,
    outbox: VecDeque<S::OutEvent>,
}

/// Public API,
impl<S: Service> BufferedContext<S> {
    /// Create a new service context for the given service.
    pub fn new(service: S) -> Self {
        Self {
            service,
            inbox: VecDeque::new(),
            outbox: VecDeque::new(),
        }
    }

    /// Get a reference to the inner service.
    pub fn service(&self) -> &S {
        &self.service
    }
}

impl<S: Service + Default> Default for BufferedContext<S> {
    fn default() -> Self {
        Self::new(Default::default())
    }
}

impl<S: Service> std::ops::Deref for BufferedContext<S> {
    type Target = S;

    fn deref(&self) -> &Self::Target {
        &self.service
    }
}

impl<S: Service> ServiceContext for BufferedContext<S> {
    type InEvent = S::InEvent;
    type OutEvent = S::OutEvent;

    /// Sends a message unconditionally, ignoring any potential errors.
    ///
    /// The event will be enqueued into the service input mailbox and will be processed by the
    /// service on the next [`BufferedContext::poll`] call. See [`poll`](#method.poll) for more
    /// details.
    fn do_send(&mut self, ev: S::InEvent) {
        self.inbox.push_back(ev);
    }

    /// Poll the service for events.
    ///
    /// The service polling process consists of the following steps:
    ///  1. Poll the service for events. This triggers the [`Service::poll`] method, allowing the
    ///     service to emit events.
    ///
    ///     If the `poll` method returns a `Poll::Ready` event, the polling process will stop any
    ///     further polling and the event will be returned to the downstream service.
    ///  2. Process all the service output mailbox events. This drains the output mailbox and
    ///     returns the events to the downstream service.
    fn poll(&mut self, cx: &mut Context<'_>) -> Poll<S::OutEvent> {
        // Poll the service for events.
        let svc_cx = BufferedPollCtx::new(&mut self.inbox, &mut self.outbox);
        if let Poll::Ready(event) = self.service.poll(svc_cx, cx) {
            return Poll::Ready(event);
        }

        // Process the outbox events.
        if let Some(ev) = self.outbox.pop_front() {
            return Poll::Ready(ev);
        }

        Poll::Pending
    }
}
