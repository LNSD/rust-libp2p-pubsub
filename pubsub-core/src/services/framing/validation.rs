//! This crate provides validation for the pubsub frame and its sub-parts.

use libp2p_pubsub_proto::pubsub::{ControlMessageProto, FrameProto};

/// Errors that can occur when validating a [`FrameProto`].
///
/// See [`validate_frame_proto`] for more details.
#[derive(Debug, Clone, Copy, thiserror::Error)]
pub enum FrameValidationError {
    /// Empty frame.
    #[error("empty frame")]
    EmptyFrame,

    /// Empty control message.
    #[error("empty control message")]
    EmptyControl,
}

/// Validates a [`FrameProto`].
///
/// A frame protobuf is valid if:
/// - It contains at least one publish, subscription or control message.
///
/// If the frame is invalid, a [`FrameValidationError`] is returned.
pub fn validate_frame_proto(rpc: &FrameProto) -> Result<(), FrameValidationError> {
    if rpc.publish.is_empty() && rpc.subscriptions.is_empty() && rpc.control.is_none() {
        // RPC frame must not be empty.
        return Err(FrameValidationError::EmptyFrame);
    }

    if rpc.publish.is_empty() && rpc.subscriptions.is_empty() {
        if let Some(control) = &rpc.control {
            // If the frame contains a control message, it must be valid.
            validate_control_proto(control)?;
        }
    }

    Ok(())
}

/// Validates a [`ControlMessageProto`].
///
/// A control message protobuf is valid if:
/// - It contains at least one IHAVE, IWANT, GRAFT or PRUNE message.
///
/// If the control message is invalid, a [`FrameValidationError`] is returned.
pub fn validate_control_proto(control: &ControlMessageProto) -> Result<(), FrameValidationError> {
    if control.ihave.is_empty()
        && control.iwant.is_empty()
        && control.graft.is_empty()
        && control.prune.is_empty()
    {
        // Control message must not be empty.
        return Err(FrameValidationError::EmptyControl);
    }

    Ok(())
}
