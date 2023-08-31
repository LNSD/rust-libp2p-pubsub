use std::collections::VecDeque;
use std::ops::Deref;
use std::task::{Context as TaskContext, Poll};

use crate::service::service_trait::Service;

pub struct Context<S: Service> {
    service: S,
    inbox: VecDeque<S::InEvent>,
    outbox: VecDeque<S::OutEvent>,
}

/// Public API,
impl<S: Service> Context<S> {
    /// Create a new service context.
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

    /// Send an event to the service.
    ///
    /// The event queued and will be processed on the next call to `poll`.
    pub fn do_send(&mut self, ev: S::InEvent) {
        self.inbox.push_back(ev);
    }

    /// Poll the service for events.
    ///
    /// The polling process is as follows:
    ///  1. Process the outbox events: If there is an event in the outbox, return it.
    ///  2. Process the mailbox messages: If there is an event in the mailbox, process it and queue
    ///     the result to the outbox.
    ///  3. Poll the service for events.
    pub fn poll(&mut self, cx: &mut TaskContext<'_>) -> Poll<S::OutEvent> {
        while let Some(in_event) = self.inbox.pop_front() {
            if let Some(out_event) = self.service.on_event(in_event) {
                self.outbox.push_back(out_event);
            }
        }

        if let Some(ev) = self.outbox.pop_front() {
            return Poll::Ready(ev);
        }

        self.service.poll(cx)
    }
}

impl<S: Service + Default> Default for Context<S> {
    fn default() -> Self {
        Self::new(S::default())
    }
}

impl<S: Service> Deref for Context<S> {
    type Target = S;

    fn deref(&self) -> &Self::Target {
        &self.service
    }
}
