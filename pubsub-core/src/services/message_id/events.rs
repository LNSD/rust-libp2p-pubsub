use std::rc::Rc;

use libp2p::identity::PeerId;

use crate::framing::Message;
use crate::message_id::{MessageId, MessageIdFn};
use crate::topic::TopicHash;

/// Message cache service input event.
#[derive(Clone)]
pub enum ServiceIn {
    /// A subscription event.
    ///
    /// It can be either a topic subscription or unsubscription.
    SubscriptionEvent(SubscriptionEvent),
    /// A message event occurred.
    MessageEvent(MessageEvent),
}

/// Node subscriptions event.
#[derive(Clone)]
pub enum SubscriptionEvent {
    /// The node subscribed to a topic.
    Subscribed {
        /// The subscribed topic.
        topic: TopicHash,
        /// The message id function.
        message_id_fn: Option<Rc<dyn MessageIdFn<Output = MessageId>>>,
    },
    /// The node unsubscribed from a topic.
    Unsubscribed(TopicHash),
}

/// A message event occurred.
#[derive(Clone)]
pub enum MessageEvent {
    /// A message was published by the local node.
    Published(Rc<Message>),
    /// A message was received from a remote peer.
    Received {
        /// The propagation node peer id.
        src: PeerId,
        /// The message.
        message: Rc<Message>,
    },
}

#[derive(Debug, Clone)]
pub enum ServiceOut {
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
