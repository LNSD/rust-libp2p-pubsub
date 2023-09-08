use libp2p_pubsub_proto::pubsub::SubOptsProto;

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

impl From<SubOptsProto> for SubscriptionAction {
    /// Convert a [`SubOptsProto`] into a [`SubscriptionAction`].
    fn from(proto: SubOptsProto) -> Self {
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
