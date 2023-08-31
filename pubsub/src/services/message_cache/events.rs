use std::rc::Rc;

use crate::framing::Message;
use crate::message_id::MessageIdFn;
use crate::TopicHash;

/// Message cache service input event.
pub enum MessageCacheInEvent {
    /// The local node subscribed to a topic.
    Subscribed {
        topic: TopicHash,
        message_id_fn: Option<Rc<MessageIdFn>>,
    },
    /// The local node unsubscribed from a topic.
    Unsubscribed(TopicHash),
    /// A message was published by the local node.
    MessagePublished(Rc<Message>),
    /// A message was received from a remote peer.
    MessageReceived(Rc<Message>),
}
