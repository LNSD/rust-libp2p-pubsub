//! This crate provides validation for the pubsub frame and its different sub-parts.
//!
//! - [`validate_frame_proto`] validates the pubsub frame ([`FrameProto`]).
//! - [`validate_message_proto`] validates the messages ([`MessageProto`]).
//! - [`validate_subopts_proto`] validates the subscription actions ([`SubOptsProto`]).
//! - [`validate_control_proto`] validates the control messages ([`ControlMessageProto`]).

use libp2p::identity::PeerId;

use pubsub_proto::pubsub::{ControlMessageProto, FrameProto, MessageProto, SubOptsProto};

/// Errors that can occur when validating a [`FrameProto`].
///
/// See [`validate_frame_proto`] for more details.
#[derive(Debug, Clone, Copy, thiserror::Error)]
pub enum FrameValidationError {
    /// Empty frame.
    #[error("empty frame")]
    EmptyFrame,
}

/// Validates a [`FrameProto`].
///
/// A frame protobuf is valid if:
/// - It contains at least one publish, subscription or control message.
///
/// If the frame is invalid, a [`FrameValidationError`] is returned.
pub fn validate_frame_proto(rpc: &FrameProto) -> Result<(), FrameValidationError> {
    if rpc.publish.is_empty()
        && rpc.subscriptions.is_empty()
        && !matches!(rpc.control.as_ref(), Some(control) if validate_control_proto(control).is_ok())
    {
        // RPC frame must not be empty.
        return Err(FrameValidationError::EmptyFrame);
    }

    Ok(())
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

/// Validates a [`MessageProto`].
///
/// A message protobuf is valid if:
/// - The `topic` is not empty.
/// - The `from` field's peer ID, if present, is valid.
///
/// If the message is invalid, a [`MessageValidationError`] is returned.
pub fn validate_message_proto(message: &MessageProto) -> Result<(), MessageValidationError> {
    if message.topic.is_empty() {
        // topic field must not be empty
        return Err(MessageValidationError::EmptyTopic);
    }

    // If present, from field must hold a valid PeerId
    if let Some(peer_id) = message.from.as_ref() {
        if PeerId::from_bytes(peer_id).is_err() {
            return Err(MessageValidationError::InvalidPeerId);
        }
    }

    Ok(())
}

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

/// Validates a [`SubOptsProto`].
///
/// A subscription option protobuf is valid if:
/// - The `topic_id` is present and not empty.
/// - The `subscribe` field is present.
///
/// If the subscription option is invalid, a [`SubOptsValidationError`] is returned.
pub fn validate_subopts_proto(subopts: &SubOptsProto) -> Result<(), SubOptsValidationError> {
    match subopts.topic_id.as_ref() {
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

    if subopts.subscribe.is_none() {
        // Action field must be present.
        return Err(SubOptsValidationError::MissingAction);
    }

    Ok(())
}

/// Errors that can occur when validating a [`ControlMessageProto`].
///
/// See [`validate_control_proto`] for more details.
#[derive(Debug, Clone, Copy, thiserror::Error)]
pub enum ControlMessageValidationError {
    /// Empty control message
    #[error("empty control message")]
    EmptyControl,
}

/// Validates a [`ControlMessageProto`].
///
/// A control message protobuf is valid if:
/// - It contains at least one IHAVE, IWANT, GRAFT or PRUNE message.
///
/// If the control message is invalid, a [`ControlMessageValidationError`] is returned.
pub fn validate_control_proto(
    control: &ControlMessageProto,
) -> Result<(), ControlMessageValidationError> {
    if control.ihave.is_empty()
        && control.iwant.is_empty()
        && control.graft.is_empty()
        && control.prune.is_empty()
    {
        // Control message must not be empty.
        return Err(ControlMessageValidationError::EmptyControl);
    }

    Ok(())
}
