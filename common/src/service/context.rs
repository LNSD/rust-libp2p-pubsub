use std::collections::VecDeque;
use std::task::{Context as TaskContext, Poll};

use crate::service::context_handles::{OnEventCtx, PollCtx};
use crate::service::service_trait::Service;

/// A service context is a wrapper around a service that can be used to send events to it and poll
/// it for events.
///
/// See [`BufferedContext`] documentation for an implementation that provides a mailbox for input
/// events and a mailbox for output events.
pub trait ServiceContext {
    /// The input event type.
    type InEvent: 'static;
    /// The output event type.
    type OutEvent: 'static;

    /// Sends a message to the service context unconditionally, ignoring any potential errors.
    fn do_send(&mut self, ev: Self::InEvent);

    /// Polls the service context for events.
    fn poll(&mut self, cx: &mut TaskContext<'_>) -> Poll<Self::OutEvent>;
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

impl<S: Service> ServiceContext for BufferedContext<S> {
    type InEvent = S::InEvent;
    type OutEvent = S::OutEvent;

    /// Sends a message unconditionally, ignoring any potential errors.
    ///
    /// The event will be enqueued into the service input mailbox and will be processed by the
    /// service on the next [`BufferedContext::poll`] call. See [`poll`](#method.poll) for more details.
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
    ///  2. Process all the service input mailbox events. This triggers the [`Service::on_event`] method
    ///     for each event, allowing the service state to be updated.
    ///  3. Process all the service output mailbox events. This drains the output mailbox and returns
    ///     the events to the downstream service.
    fn poll(&mut self, cx: &mut TaskContext<'_>) -> Poll<S::OutEvent> {
        // Poll the service for events.
        let mut svc_cx = PollCtx::new(&mut self.inbox, &mut self.outbox);
        if let Poll::Ready(event) = self.service.poll(&mut svc_cx, cx) {
            return Poll::Ready(event);
        }

        // Process the inbox events.
        let mut svc_cx = OnEventCtx::new(&mut self.outbox);
        while let Some(event) = self.inbox.pop_front() {
            self.service.on_event(&mut svc_cx, event);
        }

        // Process the outbox events.
        if let Some(ev) = self.outbox.pop_front() {
            return Poll::Ready(ev);
        }

        Poll::Pending
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
