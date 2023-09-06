use pubsub_proto::pubsub::FrameProto;

use crate::framing::message::Message;
use crate::framing::subopts::SubscriptionAction;

// TODO: Add control messages support
#[derive(Clone, Debug, Default)]
pub struct Frame {
    /// The subscriptions to add or remove.
    subscriptions: Vec<SubscriptionAction>,

    /// The messages to send.
    messages: Vec<Message>,
}

impl Frame {
    /// Creates a new empty [`Frame`].
    #[cfg(test)]
    pub fn empty() -> Self {
        Self {
            subscriptions: Vec::new(),
            messages: Vec::new(),
        }
    }

    /// Creates a new [`Frame`] with the given subscriptions.
    #[must_use]
    pub fn new_with_subscriptions(
        subscriptions: impl IntoIterator<Item = SubscriptionAction>,
    ) -> Self {
        Self {
            subscriptions: subscriptions.into_iter().collect(),
            ..Default::default()
        }
    }

    /// Creates a new [`Frame`] with the given messages.
    #[must_use]
    pub fn new_with_messages(messages: impl IntoIterator<Item = Message>) -> Self {
        Self {
            messages: messages.into_iter().collect(),
            ..Default::default()
        }
    }
}

impl From<Frame> for FrameProto {
    /// Convert a [`Frame`] into a [`FrameProto`].
    fn from(frame: Frame) -> Self {
        Self {
            subscriptions: frame.subscriptions.into_iter().map(Into::into).collect(),
            publish: frame.messages.into_iter().map(Into::into).collect(),
            control: None,
        }
    }
}
