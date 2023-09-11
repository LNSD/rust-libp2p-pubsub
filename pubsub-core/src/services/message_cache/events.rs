use std::rc::Rc;

use libp2p::identity::PeerId;

use crate::framing::Message;
use crate::message_id::MessageId;

/// Message cache service input event.
#[derive(Clone)]
pub enum ServiceIn {
    /// A message event occurred.
    MessageEvent(MessageEvent),
}

#[derive(Clone)]
pub enum MessageEvent {
    /// A message was published by the local node.
    MessagePublished {
        /// The message.
        message: Rc<Message>,
        /// The message id.
        message_id: MessageId,
    },
    /// A message was received from a remote peer.
    MessageReceived {
        /// The propagation node peer id.
        src: PeerId,
        /// The message.
        message: Rc<Message>,
        /// The message id.
        message_id: MessageId,
    },
}
