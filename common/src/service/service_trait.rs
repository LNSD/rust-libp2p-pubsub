#![allow(unused_variables)]

use std::task::{Context, Poll};

pub type InEvent<T> = <T as Service>::InEvent;
pub type OutEvent<T> = <T as Service>::OutEvent;

/// A service that can be polled for events.
pub trait Service: 'static {
    type InEvent: 'static;
    type OutEvent: 'static;

    /// Handle an event.
    fn on_event(&mut self, ev: Self::InEvent) -> Option<Self::OutEvent> {
        None
    }

    /// Poll the service for events.
    fn poll(&mut self, cx: &mut Context<'_>) -> Poll<Self::OutEvent> {
        Poll::Pending
    }
}
