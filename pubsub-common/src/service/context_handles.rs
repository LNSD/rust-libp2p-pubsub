/// A service context inbox handle.
pub trait InCtx<'a> {
    /// The input mailbox event type.
    type Event;

    /// Get the number of events in the input mailbox.
    fn len(&self) -> usize;

    /// Check if the input mailbox is empty.
    fn is_empty(&self) -> bool;

    /// Pop the next event from the input mailbox.
    fn pop_next(&mut self) -> Option<Self::Event>;
}

/// A service context outbox handle.
pub trait OutCtx<'a> {
    // The output mailbox event type.
    type Event;

    /// Emits an event to the output mailbox.
    fn emit(&mut self, ev: Self::Event);

    /// Emits a batch of events to the output mailbox.
    fn emit_batch(&mut self, evs: impl IntoIterator<Item = Self::Event>);
}
