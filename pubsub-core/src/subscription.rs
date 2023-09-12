use std::rc::Rc;

use crate::message_id::{MessageId, MessageIdFn};
use crate::topic::{Hasher, Topic, TopicHash};

#[derive(Clone)]
pub struct Subscription {
    /// The topic to subscribe to.
    pub topic: TopicHash,
    /// The message id function to use for this subscription.
    pub message_id_fn: Option<Rc<dyn MessageIdFn<Output = MessageId>>>,
}

impl std::fmt::Debug for Subscription {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Subscription")
            .field("topic", &self.topic)
            .field(
                "message_id_fn",
                match &self.message_id_fn {
                    None => &"MessageIdFn(<undefined>)",
                    Some(_) => &"MessageIdFn(<fn>)",
                },
            )
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
    message_id_fn: Option<Rc<dyn MessageIdFn<Output = MessageId>>>,
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
    /// The function takes a [`MessageRef`](crate::message_id::MessageRef) and the message's
    /// propagation peer id as inputs and outputs a byte array, [`MessageId`] to be interpreted as
    /// the message id.
    pub fn message_id_fn<F>(&mut self, id_fn: F) -> &mut Self
    where
        F: MessageIdFn + Send + Sync + 'static,
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
