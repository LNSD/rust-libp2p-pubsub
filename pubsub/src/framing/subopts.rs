use crate::framing::{validate_subopts_proto, SubOptsProto};
use crate::topic::TopicHash;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SubscriptionAction {
    /// Subscribe to a topic.
    Subscribe(TopicHash),

    /// Unsubscribe from a topic.
    Unsubscribe(TopicHash),
}

impl SubscriptionAction {
    pub fn subscribe(topic_id: impl Into<TopicHash>) -> Self {
        SubscriptionAction::Subscribe(topic_id.into())
    }

    pub fn unsubscribe(topic_id: impl Into<TopicHash>) -> Self {
        SubscriptionAction::Unsubscribe(topic_id.into())
    }
}

impl SubscriptionAction {
    /// Returns `true` if the [`SubscriptionAction`] is a [`SubscriptionAction::Subscribe`] action.
    #[must_use]
    pub fn is_subscribe(&self) -> bool {
        match self {
            SubscriptionAction::Subscribe(_) => true,
            SubscriptionAction::Unsubscribe(_) => false,
        }
    }

    /// Returns `true` if the [`SubscriptionAction`] is a [`SubscriptionAction::Unsubscribe`] action.
    pub fn is_unsubscribe(&self) -> bool {
        !self.is_subscribe()
    }

    /// Returns the topic of the [`SubscriptionAction`] action.
    #[must_use]
    pub fn topic_id(&self) -> &TopicHash {
        match self {
            SubscriptionAction::Subscribe(topic_id) => topic_id,
            SubscriptionAction::Unsubscribe(topic_id) => topic_id,
        }
    }

    /// Convert the [`SubscriptionAction`] into a [`(TopicHash, bool)`] pair.
    #[must_use]
    pub fn into_pair(self) -> (TopicHash, bool) {
        match self {
            SubscriptionAction::Subscribe(topic_id) => (topic_id, true),
            SubscriptionAction::Unsubscribe(topic_id) => (topic_id, false),
        }
    }
}

impl From<SubOptsProto> for SubscriptionAction {
    /// Convert a [`SubOptsProto`] into a [`SubscriptionAction`].
    fn from(proto: SubOptsProto) -> Self {
        debug_assert!(
            validate_subopts_proto(&proto).is_ok(),
            "invalid SubOptsProto: {proto:?}",
        );

        match proto {
            SubOptsProto {
                subscribe: Some(subscribe),
                topic_id: Some(topic_id),
            } if subscribe => SubscriptionAction::Subscribe(TopicHash::from_raw(topic_id)),
            SubOptsProto {
                subscribe: Some(subscribe),
                topic_id: Some(topic_id),
            } if !subscribe => SubscriptionAction::Unsubscribe(TopicHash::from_raw(topic_id)),
            _ => unreachable!("invalid SubOptsProto: {proto:?}"),
        }
    }
}

impl From<SubscriptionAction> for SubOptsProto {
    /// Convert a [`SubscriptionAction`] into a [`SubOptsProto`].
    fn from(action: SubscriptionAction) -> Self {
        let (topic_id, subscribe) = action.into_pair();

        Self {
            subscribe: Some(subscribe),
            topic_id: Some(topic_id.into_string()),
        }
    }
}
