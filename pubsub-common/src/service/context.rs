use std::task::{Context, Poll};

/// A service context is a wrapper around a service that can be used to send events to it and poll
/// it for events.
///
/// See [`BufferedContext`](super::buffered_context::BufferedContext) documentation for an
/// implementation that provides a mailbox for input events and a mailbox for output events.
pub trait ServiceContext {
    /// The input event type.
    type InEvent: 'static;
    /// The output event type.
    type OutEvent: 'static;

    /// Sends a message to the service context unconditionally, ignoring any potential errors.
    fn do_send(&mut self, ev: Self::InEvent);

    /// Polls the service context for events.
    fn poll(&mut self, cx: &mut Context<'_>) -> Poll<Self::OutEvent>;
}
