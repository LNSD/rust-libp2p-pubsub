use super::control::ControlMessage;
use super::message::Message;
use super::subopts::SubscriptionAction;

#[derive(Clone, Debug)]
pub struct Frame {
    /// The subscriptions to add or remove.
    pub(crate) subscriptions: Vec<SubscriptionAction>,
    /// The messages to send.
    pub(crate) messages: Vec<Message>,
    /// The control messages to send.
    pub(crate) control: Vec<ControlMessage>,
}

impl Frame {
    /// Creates a new empty [`Frame`].
    pub fn empty() -> Self {
        Self {
            subscriptions: Vec::new(),
            messages: Vec::new(),
            control: Vec::new(),
        }
    }

    /// Creates a new [`Frame`] with the given subscriptions.
    #[must_use]
    pub fn new_with_subscriptions(
        subscriptions: impl IntoIterator<Item = SubscriptionAction>,
    ) -> Self {
        Self {
            subscriptions: subscriptions.into_iter().collect(),
            ..Self::empty()
        }
    }

    /// Creates a new [`Frame`] with the given messages.
    #[must_use]
    pub fn new_with_messages(messages: impl IntoIterator<Item = Message>) -> Self {
        Self {
            messages: messages.into_iter().collect(),
            ..Self::empty()
        }
    }

    /// Creates a new [`Frame`] with the given control messages.
    #[must_use]
    pub fn new_with_control(control: impl IntoIterator<Item = ControlMessage>) -> Self {
        Self {
            control: control.into_iter().collect(),
            ..Self::empty()
        }
    }
}
