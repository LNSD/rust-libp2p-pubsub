use std::collections::vec_deque::Drain;
use std::collections::VecDeque;

/// A service context mailbox handle for the [`Service::on_event`](super::Service::on_event) method.
pub struct OnEventCtx<'a, OutEvent> {
    outbox: &'a mut VecDeque<OutEvent>,
}

impl<'a, OutEvent> OnEventCtx<'a, OutEvent> {
    /// Create a new service context mailbox handle.
    pub(super) fn new(outbox: &'a mut VecDeque<OutEvent>) -> Self {
        Self { outbox }
    }

    /// Emit an event to the output mailbox.
    pub fn emit(&mut self, ev: OutEvent) {
        self.outbox.push_back(ev);
    }

    /// Emit a batch of events to the output mailbox.
    pub fn emit_batch(&mut self, ev: impl IntoIterator<Item = OutEvent>) {
        self.outbox.extend(ev);
    }
}

/// A service context mailbox handle for the [`Service::poll`](super::Service::poll) method.
pub struct PollCtx<'a, InEvent, OutEvent> {
    inbox: &'a mut VecDeque<InEvent>,
    outbox: &'a mut VecDeque<OutEvent>,
}

impl<'a, InEvent, OutEvent> PollCtx<'a, InEvent, OutEvent> {
    /// Create a new service context mailbox handle.
    pub(super) fn new(
        inbox: &'a mut VecDeque<InEvent>,
        outbox: &'a mut VecDeque<OutEvent>,
    ) -> Self {
        Self { inbox, outbox }
    }

    /// Get the number of events in the input mailbox.
    pub fn len(&self) -> usize {
        self.inbox.len()
    }

    /// Check if the input mailbox is empty.
    pub fn is_empty(&self) -> bool {
        self.inbox.is_empty()
    }

    /// Pop the next event from the input mailbox.
    pub fn pop_next(&mut self) -> Option<InEvent> {
        self.inbox.pop_front()
    }

    /// Get a `Drain` iterator over the input mailbox events.
    ///
    /// This is useful for draining the input mailbox in a loop. If the returned iterator is
    /// dropped before it is fully consumed, the remaining events will be dropped as well.
    pub fn drain(&mut self) -> Drain<'_, InEvent> {
        self.inbox.drain(..)
    }

    /// Emit an event to the output mailbox.
    pub fn emit(&mut self, ev: OutEvent) {
        self.outbox.push_back(ev);
    }

    /// Emit a batch of events to the output mailbox.
    pub fn emit_batch(&mut self, evs: impl IntoIterator<Item = OutEvent>) {
        self.outbox.extend(evs);
    }
}
