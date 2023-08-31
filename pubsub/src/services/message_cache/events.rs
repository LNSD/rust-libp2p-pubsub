use std::fmt::{Debug, Formatter};
use std::rc::Rc;

use crate::framing::Message;
use crate::message_id::MessageIdFn;
use crate::topic::TopicHash;

/// Message cache service input event.
#[derive(Debug, Clone)]
pub enum ServiceIn {
    /// A subscription event.
    SubscriptionEvent(SubscriptionEvent),
    /// A message was published by the local node.
    MessagePublished(Rc<Message>),
    /// A message was received from a remote peer.
    MessageReceived(Rc<Message>),
}

impl ServiceIn {
    /// Create a new `ServiceIn::SubscriptionEvent` event.
    pub fn from_subscription_event(ev: impl Into<SubscriptionEvent>) -> Self {
        ServiceIn::SubscriptionEvent(ev.into())
    }
}

/// Node subscriptions event.
#[derive(Clone)]
pub enum SubscriptionEvent {
    /// The node subscribed to a topic.
    Subscribed {
        topic: TopicHash,
        message_id_fn: Option<Rc<MessageIdFn>>,
    },
    /// The node unsubscribed from a topic.
    Unsubscribed(TopicHash),
}

impl Debug for SubscriptionEvent {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SubscriptionEvent::Subscribed {
                topic,
                message_id_fn,
            } => f
                .debug_struct("Subscribed")
                .field("topic", topic)
                .field(
                    "message_id_fn",
                    match message_id_fn {
                        None => &"MessageIdFn(default)",
                        Some(_) => &"MessageIdFn(custom)",
                    },
                )
                .finish(),
            SubscriptionEvent::Unsubscribed(topic) => f
                .debug_struct("Unsubscribed")
                .field("topic", topic)
                .finish(),
        }
    }
}
