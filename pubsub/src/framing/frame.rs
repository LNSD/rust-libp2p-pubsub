use crate::framing::message::Message;
use crate::framing::subopts::SubscriptionAction;
use crate::framing::FrameProto;

// TODO: Add control messages support
#[derive(Clone, Debug, Default)]
pub struct Frame {
    /// The subscriptions to add or remove.
    subscriptions: Vec<SubscriptionAction>,

    /// The messages to send.
    messages: Vec<Message>,
}

impl Frame {
    /// Creates a new [`Frame`] with the given subscriptions and messages.
    #[must_use]
    pub fn new(
        subscriptions: impl IntoIterator<Item = SubscriptionAction>,
        messages: impl IntoIterator<Item = Message>,
    ) -> Self {
        Self {
            subscriptions: subscriptions.into_iter().collect(),
            messages: messages.into_iter().collect(),
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

    /// Returns the subscriptions to add or remove.
    #[must_use]
    pub fn subscriptions(&self) -> &[SubscriptionAction] {
        &self.subscriptions
    }

    /// Returns the messages to send.
    #[must_use]
    pub fn messages(&self) -> &[Message] {
        &self.messages
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
