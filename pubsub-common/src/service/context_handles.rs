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

/// A service context input and output mailbox handles.
pub trait JointCtx<'a, InEvent, OutEvent> {
    /// The input mailbox handle type.
    type InHandle: InCtx<'a, Event = InEvent> + 'a;
    /// The output mailbox handle type.
    type OutHandle: OutCtx<'a, Event = OutEvent> + 'a;

    /// Split this context into its input and output mailbox context handles.
    fn split(self) -> (Self::InHandle, Self::OutHandle);
}

// NOTE: Use `trait_set` crate as `trait_alias` is not yet stable.
//       https://github.com/rust-lang/rust/issues/41517
trait_set::trait_set! {
    /// A service context mailbox handle for the [`Service::on_event`](super::Service::on_event) method.
    pub trait OnEventCtx<'a, OutEvent> = OutCtx<'a, Event = OutEvent>;

    /// A service context mailbox handle for the [`Service::poll`](super::Service::poll) method.
    pub trait PollCtx<'a, InEvent, OutEvent> = JointCtx<'a, InEvent, OutEvent>
        + InCtx<'a, Event = InEvent>
        + OutCtx<'a, Event = OutEvent>;
}
