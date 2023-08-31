use std::fmt::{Debug, Formatter};
use std::rc::Rc;

use crate::framing::Message;
use crate::message_id::{MessageId, MessageIdFn};
use crate::topic::{Hasher, Topic, TopicHash};

#[derive(Clone)]
pub struct Subscription {
    /// The topic to subscribe to.
    pub topic: TopicHash,

    /// The message id function to use for this subscription.
    pub message_id_fn: Option<Rc<MessageIdFn>>,
}

impl Debug for Subscription {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Subscription")
            .field("topic", &self.topic)
            .field("message_id_fn", &self.message_id_fn.is_some())
            .finish()
    }
}

impl From<TopicHash> for Subscription {
    fn from(topic: TopicHash) -> Self {
        Self {
            topic,
            message_id_fn: None,
        }
    }
}

impl<H: Hasher> From<Topic<H>> for Subscription {
    fn from(topic: Topic<H>) -> Self {
        topic.hash().into()
    }
}

/// A builder for a subscription.
pub struct SubscriptionBuilder {
    topic: TopicHash,
    message_id_fn: Option<Rc<MessageIdFn>>,
}

impl SubscriptionBuilder {
    /// Create a new subscription builder.
    pub fn new<H: Hasher>(topic: Topic<H>) -> Self {
        Self {
            topic: topic.hash(),
            message_id_fn: None,
        }
    }

    /// A user-defined function allowing the user to specify the message id of a pub-sub message.
    /// The default value is to concatenate the source peer id with a sequence number. Setting this
    /// parameter allows the user to address packets arbitrarily. One example is content based
    /// addressing, where this function may be set to `hash(message)`. This would prevent messages
    /// of the same content from being duplicated.
    ///
    /// The function takes a [`Message`] as input and outputs a String to be interpreted as the
    /// message id.
    pub fn message_id_fn<F>(&mut self, id_fn: F) -> &mut Self
    where
        F: Fn(&Message) -> MessageId + Send + Sync + 'static,
    {
        self.message_id_fn = Some(Rc::new(id_fn));
        self
    }

    pub fn build(self) -> Subscription {
        Subscription {
            topic: self.topic,
            message_id_fn: self.message_id_fn,
        }
    }
}
