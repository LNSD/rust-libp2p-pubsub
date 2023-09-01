use libp2p::identity::PeerId;

// Re-export temporarily the `Message` type from `framing` until we have a proper message type
// TODO: Add a different message type for the application events
pub use crate::framing::Message;

/// Events emitted by the pubsub behaviour.
#[derive(Debug)]
pub enum Event {
    /// Message received.
    MessageReceived {
        /// Peer that propagated the message.
        src: PeerId,
        /// The message.
        message: Message,
    },
}
