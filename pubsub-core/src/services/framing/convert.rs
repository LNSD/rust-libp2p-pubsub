//! Traits for converting between [`framing`](crate::framing) types and
//! [libp2p pubsub protobuf](libp2p_pubsub_proto::pubsub) types.

use bytes::Bytes;

use libp2p_pubsub_proto::pubsub::{FrameProto, MessageProto, SubOptsProto};

use crate::framing::{Frame, Message, SubscriptionAction};
use crate::topic::TopicHash;

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

impl From<MessageProto> for Message {
    /// Convert from a [`MessageProto`] into a [`Message`].
    ///
    /// Additionally. sanitize the protobuf message by removing optional fields when empty.
    #[must_use]
    fn from(mut proto: MessageProto) -> Self {
        // A non-present data field should be interpreted as an empty payload.
        if proto.data.is_none() {
            proto.data = Some(Bytes::new());
        }

        // An empty from field should be interpreted as not present.
        if let Some(from) = proto.from.as_ref() {
            if from.is_empty() {
                proto.from = None;
            }
        }

        // An empty seqno field should be interpreted as not present.
        if let Some(seq_no) = proto.seqno.as_ref() {
            if seq_no.is_empty() {
                proto.seqno = None;
            }
        }

        // An empty signature field should be interpreted as not present.
        if let Some(signature) = proto.signature.as_ref() {
            if signature.is_empty() {
                proto.signature = None;
            }
        }

        // An empty key field should be interpreted as not present.
        if let Some(key) = proto.key.as_ref() {
            if key.is_empty() {
                proto.key = None;
            }
        }

        Self { proto }
    }
}

impl From<Message> for MessageProto {
    /// Convert a [`Message`] into a [`MessageProto`].
    fn from(message: Message) -> Self {
        message.into_proto()
    }
}
impl From<Frame> for FrameProto {
    /// Convert a [`Frame`] into a [`FrameProto`].
    fn from(frame: Frame) -> Self {
        // Convert the subscriptions into a protobuf message.
        let subscriptions = frame.subscriptions.into_iter().map(Into::into).collect();

        // Convert the messages into a protobuf message.
        let publish = frame.messages.into_iter().map(Into::into).collect();

        // TODO: Convert the control messages into a protobuf message.
        let control = None;

        Self {
            subscriptions,
            publish,
            control,
        }
    }
}
