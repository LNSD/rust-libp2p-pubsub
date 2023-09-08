use libp2p::identity::PeerId;

use crate::message::Message;

/// This enum represents events that can be emitted by the pubsub
/// [`Behaviour`](super::behaviour::Behaviour).
#[derive(Debug)]
pub enum Event {
    /// Emitted by the pubsub behaviour when a message associated with a topic the node is
    /// subscribed to is received.
    MessageReceived {
        /// Peer that propagated the message.
        ///
        /// Do not confuse with the original author of the message, which is optionally included in
        /// the message itself in the message's `from` field.
        /// field.
        src: PeerId,
        /// The message itself.
        message: Message,
    },
}
