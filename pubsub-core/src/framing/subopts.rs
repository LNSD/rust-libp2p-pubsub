use crate::topic::TopicHash;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SubscriptionAction {
    /// Subscribe to a topic.
    Subscribe(TopicHash),

    /// Unsubscribe from a topic.
    Unsubscribe(TopicHash),
}

impl SubscriptionAction {
    /// Convert the [`SubscriptionAction`] into a [`(TopicHash, bool)`] pair.
    #[must_use]
    pub fn into_pair(self) -> (TopicHash, bool) {
        match self {
            SubscriptionAction::Subscribe(topic_id) => (topic_id, true),
            SubscriptionAction::Unsubscribe(topic_id) => (topic_id, false),
        }
    }
}
