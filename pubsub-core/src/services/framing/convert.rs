//! Traits for converting between [`framing`](crate::framing) types and
//! [libp2p pubsub protobuf](libp2p_pubsub_proto::pubsub) types.

use bytes::Bytes;
use libp2p::PeerId;

use libp2p_pubsub_proto::pubsub::{
    ControlGraftProto, ControlIHaveProto, ControlIWantProto, ControlMessageProto,
    ControlPruneProto, FrameProto, MessageProto, SubOptsProto,
};

use crate::framing::{
    ControlMessage, Frame, GraftControlMessage, IHaveControlMessage, IWantControlMessage, Message,
    PruneControlMessage, SubscriptionAction,
};
use crate::message_id::MessageId;
use crate::topic::TopicHash;

/// Errors that can occur when validating a [`SubOptsProto`].
///
/// See [`validate_subopts_proto`] for more details.
#[derive(Debug, Clone, Copy, thiserror::Error)]
pub enum SubOptsValidationError {
    /// Empty message topic.
    #[error("empty topic")]
    EmptyTopic,

    /// Topic not present.
    #[error("topic not present")]
    MissingTopic,

    /// Action not present.
    #[error("subscription action not present")]
    MissingAction,
}

impl TryFrom<SubOptsProto> for SubscriptionAction {
    type Error = SubOptsValidationError;

    /// Convert a [`SubOptsProto`] into a [`SubscriptionAction`].
    ///
    /// A subscription option protobuf is valid if:
    /// - The `topic_id` is present and not empty.
    /// - The `subscribe` field is present.
    ///
    /// If the subscription option is invalid, a [`SubOptsValidationError`] is returned.
    fn try_from(proto: SubOptsProto) -> Result<Self, Self::Error> {
        match proto.topic_id.as_ref() {
            None => {
                // Topic field must be present.
                return Err(SubOptsValidationError::MissingTopic);
            }
            Some(topic) if topic.is_empty() => {
                // Topic field must not be empty.
                return Err(SubOptsValidationError::EmptyTopic);
            }
            _ => {}
        }

        if proto.subscribe.is_none() {
            // Action field must be present.
            return Err(SubOptsValidationError::MissingAction);
        }

        let action = if proto.subscribe.unwrap() {
            SubscriptionAction::Subscribe(TopicHash::from_raw(proto.topic_id.unwrap()))
        } else {
            SubscriptionAction::Unsubscribe(TopicHash::from_raw(proto.topic_id.unwrap()))
        };

        Ok(action)
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

/// Errors that can occur when validating a [`MessageProto`].
///
/// See [`validate_message_proto`] for more details.
#[derive(Debug, Clone, Copy, thiserror::Error)]
pub enum MessageValidationError {
    /// Empty message topic.
    #[error("empty topic")]
    EmptyTopic,
    /// The message source was invalid (invalid peer ID).
    #[error("invalid peer id")]
    InvalidPeerId,
}

impl TryFrom<MessageProto> for Message {
    type Error = MessageValidationError;

    /// Convert from a [`MessageProto`] into a [`Message`].
    ///
    /// A message protobuf is valid if:
    /// - The `topic` is not empty.
    /// - The `from` field's peer ID, if present, is valid.
    ///
    /// Additionally. sanitize the protobuf message by removing optional fields when empty.
    fn try_from(mut proto: MessageProto) -> Result<Self, Self::Error> {
        if proto.topic.is_empty() {
            // topic field must not be empty
            return Err(MessageValidationError::EmptyTopic);
        }

        // A non-present data field should be interpreted as an empty payload.
        if proto.data.is_none() {
            proto.data = Some(Bytes::new());
        }

        // An empty from field should be interpreted as not present.
        match proto.from.as_ref() {
            Some(from) if from.is_empty() => {
                proto.from = None;
            }
            Some(from) => {
                // If present, from field must hold a valid PeerId
                if PeerId::from_bytes(from).is_err() {
                    return Err(MessageValidationError::InvalidPeerId);
                }
            }
            None => {}
        }

        // An empty seq_no field should be interpreted as not present.
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

        Ok(Self { proto })
    }
}

impl From<Message> for MessageProto {
    /// Convert a [`Message`] into a [`MessageProto`].
    fn from(message: Message) -> Self {
        message.into_proto()
    }
}

/// Validation errors for converting a [`ControlGraftProto`] into a [`GraftControlMessage`].
#[derive(Debug, thiserror::Error)]
pub enum ControlGraftMessageError {
    #[error("topic_id not present")]
    TopicIdNotPresent,

    #[error("empty topic_id")]
    EmptyTopicId,
}

impl TryFrom<ControlGraftProto> for GraftControlMessage {
    type Error = ControlGraftMessageError;

    /// Convert a [`ControlGraftProto`] into a [`ControlMessage`].
    fn try_from(value: ControlGraftProto) -> Result<Self, Self::Error> {
        let topic_hash = match value.topic_id {
            None => return Err(ControlGraftMessageError::TopicIdNotPresent),
            Some(topic) if topic.is_empty() => return Err(ControlGraftMessageError::EmptyTopicId),
            Some(topic) => topic.into(),
        };

        Ok(GraftControlMessage { topic_hash })
    }
}

impl From<GraftControlMessage> for ControlGraftProto {
    /// Convert a [`ControlMessage`] into a [`ControlGraftProto`].
    fn from(value: GraftControlMessage) -> Self {
        Self {
            topic_id: Some(value.topic_hash.into_string()),
        }
    }
}

/// Validation errors for converting a [`ControlPruneProto`] into a [`PruneControlMessage`].
#[derive(Debug, thiserror::Error)]
pub enum ControlPruneMessageError {
    #[error("topic_id not present")]
    TopicIdNotPresent,

    #[error("empty topic_id")]
    EmptyTopicId,
}

impl TryFrom<ControlPruneProto> for PruneControlMessage {
    type Error = ControlPruneMessageError;

    /// Convert a [`ControlPruneProto`] into a [`PruneControlMessage`].
    fn try_from(value: ControlPruneProto) -> Result<Self, Self::Error> {
        let topic_hash = match value.topic_id {
            None => return Err(ControlPruneMessageError::TopicIdNotPresent),
            Some(topic) if topic.is_empty() => return Err(ControlPruneMessageError::EmptyTopicId),
            Some(topic) => topic.into(),
        };

        let backoff = value.backoff;

        Ok(PruneControlMessage {
            topic_hash,
            peers: vec![], // TODO: Add support for peer exchange.
            backoff,
        })
    }
}

impl From<PruneControlMessage> for ControlPruneProto {
    /// Convert a [`ControlMessage`] into a [`ControlPruneProto`].
    fn from(value: PruneControlMessage) -> Self {
        Self {
            topic_id: Some(value.topic_hash.into_string()),
            peers: vec![], // TODO: Add support for peer exchange.
            backoff: value.backoff,
        }
    }
}

/// Validation errors for converting a [`ControlIWantProto`] into a [`IWantControlMessage`].
#[derive(Debug, thiserror::Error)]
pub enum ControlIWantMessageError {
    #[error("empty message_ids list")]
    EmptyMessageIdsList,
}

impl TryFrom<ControlIWantProto> for IWantControlMessage {
    type Error = ControlIWantMessageError;

    /// Convert a [`ControlIWantProto`] into a [`ControlMessage`].
    fn try_from(value: ControlIWantProto) -> Result<Self, Self::Error> {
        let message_ids = value
            .message_ids
            .into_iter()
            .filter(|id| !id.is_empty())
            .map(MessageId::new)
            .collect::<Vec<_>>();

        if message_ids.is_empty() {
            return Err(ControlIWantMessageError::EmptyMessageIdsList);
        }

        Ok(IWantControlMessage { message_ids })
    }
}

impl From<IWantControlMessage> for ControlIWantProto {
    /// Convert a [`ControlMessage`] into a [`ControlIWantProto`].
    fn from(value: IWantControlMessage) -> Self {
        Self {
            message_ids: value.message_ids.into_iter().map(Into::into).collect(),
        }
    }
}

/// Validation errors for converting a [`ControlIHaveProto`] into a [`IHaveControlMessage`].
#[derive(Debug, thiserror::Error)]
pub enum ControlIHaveMessageError {
    #[error("topic_id not present")]
    TopicIdNotPresent,

    #[error("empty topic_id")]
    EmptyTopicId,

    #[error("empty message_ids list")]
    EmptyMessageIdsList,
}

impl TryFrom<ControlIHaveProto> for IHaveControlMessage {
    type Error = ControlIHaveMessageError;

    /// Convert a [`ControlIHaveProto`] into a [`ControlMessage`].
    fn try_from(value: ControlIHaveProto) -> Result<Self, Self::Error> {
        let topic_hash = match value.topic_id {
            None => return Err(ControlIHaveMessageError::TopicIdNotPresent),
            Some(topic) if topic.is_empty() => return Err(ControlIHaveMessageError::EmptyTopicId),
            Some(topic) => topic.into(),
        };

        let message_ids = value
            .message_ids
            .into_iter()
            .filter(|id| !id.is_empty())
            .map(MessageId::new)
            .collect::<Vec<_>>();

        if message_ids.is_empty() {
            return Err(ControlIHaveMessageError::EmptyMessageIdsList);
        }

        Ok(IHaveControlMessage {
            topic_hash,
            message_ids,
        })
    }
}

impl From<IHaveControlMessage> for ControlIHaveProto {
    /// Convert a [`ControlMessage`] into a [`ControlIHaveProto`].
    fn from(value: IHaveControlMessage) -> Self {
        Self {
            topic_id: Some(value.topic_hash.into_string()),
            message_ids: value.message_ids.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<Frame> for FrameProto {
    /// Convert a [`Frame`] into a [`FrameProto`].
    fn from(frame: Frame) -> Self {
        // Convert the subscriptions into a protobuf message.
        let subscriptions = frame.subscriptions.into_iter().map(Into::into).collect();

        // Convert the messages into a protobuf message.
        let publish = frame.messages.into_iter().map(Into::into).collect();

        // Convert the control messages into a protobuf message.
        let control = if frame.control.is_empty() {
            None
        } else {
            Some(frame.control.into_iter().map(Into::into).fold(
                ControlMessageProto::default(),
                |mut acc, ctl_msg| {
                    match ctl_msg {
                        ControlMessage::Graft(ctl) => {
                            acc.graft.push(ctl.into());
                        }
                        ControlMessage::Prune(ctl) => {
                            acc.prune.push(ctl.into());
                        }
                        ControlMessage::IHave(ctl) => {
                            acc.ihave.push(ctl.into());
                        }
                        ControlMessage::IWant(ctl) => {
                            acc.iwant.push(ctl.into());
                        }
                    }

                    acc
                },
            ))
        };

        Self {
            subscriptions,
            publish,
            control,
        }
    }
}
