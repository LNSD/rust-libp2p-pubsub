use std::collections::VecDeque;
use std::task::{Context as TaskContext, Poll};

use crate::service::context_handles::{OnEventCtx, PollCtx};
use crate::service::service_trait::Service;

/// A service context.
///
/// The service context is a wrapper around a service that provides a mailbox for input events and
/// a mailbox for output events. It is in charge of polling the service for events and processing
/// the mailbox events.
///
/// The service context implements `Deref` trait to the inner service, so it can be used as a
/// service.
pub struct Context<S: Service> {
    service: S,
    inbox: VecDeque<S::InEvent>,
    outbox: VecDeque<S::OutEvent>,
}

/// Public API,
impl<S: Service> Context<S> {
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

    /// Sends a message unconditionally, ignoring any potential errors.
    ///
    /// The event will be enqueued into the service input mailbox and will be processed by the
    /// service on the next [`Context::poll`] call. See [`poll`](#method.poll) for more details.
    pub fn do_send(&mut self, ev: S::InEvent) {
        self.inbox.push_back(ev);
    }

    /// Poll the service for events.
    ///
    /// The polling polling strategy is as follows:
    ///
    ///  1. Process all the service input mailbox events (calls the [`Service::on_event`] method).
    ///  2. Poll the service for events (calls the [`Service::poll`] method).
    ///  3. Process all the service output mailbox events.
    ///
    /// See [`Service`](super::Service) documentation for a more detailed explanation of the service
    /// polling strategy.
    pub fn poll(&mut self, cx: &mut TaskContext<'_>) -> Poll<S::OutEvent> {
        // Process the inbox events.
        let mut svc_cx = OnEventCtx::new(&mut self.outbox);
        while let Some(event) = self.inbox.pop_front() {
            self.service.on_event(&mut svc_cx, event);
        }

        // Poll the service for events.
        let mut svc_cx = PollCtx::new(&mut self.inbox, &mut self.outbox);
        if let Poll::Ready(event) = self.service.poll(&mut svc_cx, cx) {
            return Poll::Ready(event);
        }

        // Process the outbox events.
        if let Some(ev) = self.outbox.pop_front() {
            return Poll::Ready(ev);
        }

        Poll::Pending
    }
}

impl<S: Service + Default> Default for Context<S> {
    fn default() -> Self {
        Self::new(Default::default())
    }
}

impl<S: Service> std::ops::Deref for Context<S> {
    type Target = S;

    fn deref(&self) -> &Self::Target {
        &self.service
    }
}
