use std::collections::VecDeque;

/// A service context inbox handle.
pub struct InCtx<'a, InEvent> {
    inbox: &'a mut VecDeque<InEvent>,
}

impl<'a, InEvent> InCtx<'a, InEvent> {
    /// Create a new service context inbox handle.
    pub(super) fn new(inbox: &'a mut VecDeque<InEvent>) -> Self {
        Self { inbox }
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
}

/// A service context outbox handle.
pub struct OutCtx<'a, OutEvent> {
    outbox: &'a mut VecDeque<OutEvent>,
}

impl<'a, OutEvent> OutCtx<'a, OutEvent> {
    /// Create a new service context outbox handle.
    pub(super) fn new(outbox: &'a mut VecDeque<OutEvent>) -> Self {
        Self { outbox }
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

/// A service context mailbox handle for the [`Service::on_event`](super::Service::on_event) method.
pub struct OnEventCtx<'a, OutEvent> {
    outbox: OutCtx<'a, OutEvent>,
}

impl<'a, OutEvent> OnEventCtx<'a, OutEvent> {
    /// Emit an event to the output mailbox.
    pub fn emit(&mut self, ev: OutEvent) {
        self.outbox.emit(ev);
    }

    /// Emit a batch of events to the output mailbox.
    pub fn emit_batch(&mut self, ev: impl IntoIterator<Item = OutEvent>) {
        self.outbox.emit_batch(ev);
    }
}

impl<'a, OutEvent> From<OutCtx<'a, OutEvent>> for OnEventCtx<'a, OutEvent> {
    fn from(value: OutCtx<'a, OutEvent>) -> Self {
        Self { outbox: value }
    }
}

/// A service context mailbox handle for the [`Service::poll`](super::Service::poll) method.
pub struct PollCtx<'a, InEvent, OutEvent> {
    inbox: InCtx<'a, InEvent>,
    outbox: OutCtx<'a, OutEvent>,
}

impl<'a, InEvent, OutEvent> PollCtx<'a, InEvent, OutEvent> {
    /// Create a new service context mailbox handle.
    pub(super) fn new(
        inbox: &'a mut VecDeque<InEvent>,
        outbox: &'a mut VecDeque<OutEvent>,
    ) -> Self {
        Self {
            inbox: InCtx::new(inbox),
            outbox: OutCtx::new(outbox),
        }
    }

    /// Split this context into its input and output mailbox context handles.
    pub fn split(self) -> (InCtx<'a, InEvent>, OutCtx<'a, OutEvent>) {
        (self.inbox, self.outbox)
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
        self.inbox.pop_next()
    }

    /// Emit an event to the output mailbox.
    pub fn emit(&mut self, ev: OutEvent) {
        self.outbox.emit(ev);
    }

    /// Emit a batch of events to the output mailbox.
    pub fn emit_batch(&mut self, evs: impl IntoIterator<Item = OutEvent>) {
        self.outbox.emit_batch(evs);
    }
}
