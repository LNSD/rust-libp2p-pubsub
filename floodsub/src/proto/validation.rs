use libp2p::PeerId;

use crate::proto::{ControlMessageProto, MessageProto, RpcProto, SubOptsProto};

#[derive(Debug, Clone, Copy, thiserror::Error)]
pub enum MessageValidationError {
    /// Empty message topic.
    #[error("empty topic")]
    EmptyTopic,
    /// The message source was invalid (invalid peer id).
    #[error("invalid peer id")]
    InvalidPeerId,
    /// The sequence number was the incorrect size.
    #[error("incorrect size sequence number")]
    InvalidSequenceNumber,
}

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

    // If present, seqno field must be a 64-bit big-endian serialized unsigned integer
    if let Some(seq_no) = message.seqno.as_ref() {
        if seq_no.len() != 8 {
            return Err(MessageValidationError::InvalidSequenceNumber);
        }
    }

    Ok(())
}

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

#[derive(Debug, Clone, Copy, thiserror::Error)]
pub enum ControlMessageValidationError {
    /// Empty control message
    #[error("empty control message")]
    EmptyControl,
}

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

#[derive(Debug, Clone, Copy, thiserror::Error)]
pub enum RpcValidationError {
    /// Empty rpc frame.
    #[error("empty RPC frame")]
    EmptyRpc,
}

pub fn validate_rpc_proto(rpc: &RpcProto) -> Result<(), RpcValidationError> {
    if rpc.publish.is_empty()
        && rpc.subscriptions.is_empty()
        && !matches!(rpc.control.as_ref(), Some(control) if validate_control_proto(control).is_ok())
    {
        // RPC frame must not be empty.
        return Err(RpcValidationError::EmptyRpc);
    }

    Ok(())
}
