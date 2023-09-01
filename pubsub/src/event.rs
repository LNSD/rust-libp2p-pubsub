use libp2p::identity::PeerId;

use crate::framing::Message;

/// Events emitted by the pubsub behaviour.
#[derive(Debug)]
pub enum Event {
    /// Message received.
    MessageReceived {
        /// Peer that propagated the message.
        source: PeerId,
        /// The message.
        message: Message, // TODO: Add a different message type for the application events
    },
}
